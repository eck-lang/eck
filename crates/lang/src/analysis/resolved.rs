//! Compositional typed-IR traversal and loop-carried dependence proofs.

use std::collections::HashSet;

use crate::ir::{
    BindingId, LocalVariableSlot, TypedBinaryExecutionPlan, TypedBlock, TypedComparisonPlan,
    TypedExpression, TypedExpressionKind, TypedIndexDispatch, TypedProgram, TypedScalePlan,
    TypedStatement,
};
use crate::semantic::{
    BinaryOperator, FunctionDeterminism, FunctionExternalEffect, FunctionPurity, IndexExtractor,
    OperatorId, Registry, SemanticType,
};
use crate::syntax::Span;

use super::*;

mod builtins;
use builtins::Builtins;

/// Traverses resolved regions while retaining each loop's own iteration identity.
struct Analyzer<'registry> {
    registry: &'registry Registry,
    builtins: Builtins<'registry>,
    result: ExecutionAnalysis,
    iteration: Option<BindingResource>,
}

/// Analyzes one resolved program without changing its IR or execution behavior.
pub(super) fn analyze(program: &TypedProgram, registry: &Registry) -> ExecutionAnalysis {
    let mut analyzer = Analyzer {
        registry,
        builtins: Builtins::new(registry),
        result: ExecutionAnalysis::default(),
        iteration: None,
    };
    analyzer.result.effects = analyzer.statements(&program.statements);
    analyzer.result
}

impl Analyzer<'_> {
    /// Combines statement effects in a lexical execution sequence.
    fn statements(&mut self, statements: &[TypedStatement]) -> EffectSummary {
        let mut effects = EffectSummary::default();
        for statement in statements {
            effects.combine(&self.statement(statement));
        }
        effects
    }

    /// Records one block's compositional effects and lexical capture purity.
    fn block(&mut self, block: &TypedBlock) -> EffectSummary {
        let effects = self.statements(&block.statements);
        let local_slots = owned_slots(&block.statements);
        let pure = effects.is_pure()
            && effects
                .writes
                .iter()
                .all(|access| local_slots.contains(&access.resource.slot()));
        self.result.blocks.push(BlockAnalysis {
            span: block.span,
            effects: effects.clone(),
            pure,
        });
        effects
    }

    /// Visits a resolved statement, preserving all branch effects and capture mutations.
    fn statement(&mut self, statement: &TypedStatement) -> EffectSummary {
        let mut effects = EffectSummary::default();
        match statement {
            TypedStatement::VariableDeclaration {
                binding,
                slot,
                expression,
                span,
                ..
            }
            | TypedStatement::Assignment {
                binding,
                slot,
                expression,
                span,
                ..
            } => {
                effects.combine(&self.expression(expression));
                effects.writes.push(access(
                    Resource::Binding(binding_resource(*binding, *slot)),
                    *span,
                ));
            }
            TypedStatement::IndexedAssignment {
                binding,
                slot,
                index,
                index_extractor,
                index_dispatch,
                expression,
                span,
                ..
            } => {
                effects.combine(&self.expression(index));
                effects.combine(&self.expression(expression));
                self.index_callbacks(
                    index,
                    *index_extractor,
                    index_dispatch.as_ref(),
                    &mut effects,
                );
                effects.may_fail = true;
                effects.writes.push(access(
                    Resource::ArrayElement {
                        container: binding_resource(*binding, *slot),
                        index: self.index(index),
                    },
                    *span,
                ));
            }
            TypedStatement::MapIndexedAssignment {
                slot,
                key,
                expression,
                span,
                ..
            } => {
                effects.combine(&self.expression(key));
                effects.combine(&self.expression(expression));
                effects
                    .writes
                    .push(access(Resource::Map { slot: *slot }, *span));
                effects.reject(SequentialReason::MapOperation);
                effects.may_fail = true;
            }
            TypedStatement::Block(block) => effects.combine(&self.block(block)),
            TypedStatement::If {
                condition,
                body,
                else_body,
                ..
            } => {
                effects.combine(&self.expression(condition));
                self.boolean_callback(&mut effects);
                effects.combine(&self.block(body));
                if let Some(block) = else_body {
                    effects.combine(&self.block(block));
                }
            }
            TypedStatement::For {
                binding,
                slot,
                variable_type,
                start,
                end,
                range_plan,
                body,
                span,
                ..
            } => {
                effects.combine(&self.expression(start));
                effects.combine(&self.expression(end));
                let iteration = binding_resource(*binding, *slot);
                let previous_iteration = self.iteration.replace(iteration);
                let mut body_effects = self.block(body);
                self.iteration = previous_iteration;
                if !self.builtins.signed_integer(*variable_type)
                    || !self.builtins.operator(range_plan.increment.operator)
                    || !self.builtins.comparison(range_plan.comparison.comparison)
                    || !range_plan.increment.left_operand_scale.is_identity()
                    || !range_plan.increment.right_operand_scale.is_identity()
                    || range_plan.increment.relative_adjustment.is_some()
                    || !range_plan.comparison.left_operand_scale.is_identity()
                    || !range_plan.comparison.right_operand_scale.is_identity()
                {
                    body_effects.reject(SequentialReason::UnsupportedRange);
                }
                let mut local_slots = owned_slots(&body.statements);
                local_slots.insert(*slot);
                let analysis = classify_loop(*span, body_effects, iteration, &local_slots);
                effects.combine(&analysis.effects);
                let key = (span.start, span.end);
                if let Some(existing) = self.result.loops.get_mut(&key) {
                    existing.parallelism = Parallelism::Sequential {
                        reasons: vec![SequentialReason::DuplicateLoopSpan],
                    };
                    existing.effects.reject(SequentialReason::DuplicateLoopSpan);
                } else {
                    self.result.loops.insert(key, analysis);
                }
                effects
                    .writes
                    .push(access(Resource::Binding(iteration), *span));
                effects.reject(SequentialReason::NestedLoop);
                effects.may_fail = true;
            }
            TypedStatement::While {
                condition, body, ..
            } => {
                effects.combine(&self.expression(condition));
                self.boolean_callback(&mut effects);
                effects.combine(&self.block(body));
                effects.reject(SequentialReason::NestedLoop);
            }
            TypedStatement::ForEach { source, body, .. } => {
                effects.combine(&self.expression(source));
                effects.combine(&self.block(body));
                effects.reject(SequentialReason::UnsupportedConstruct);
                effects.reject(SequentialReason::NestedLoop);
                effects.external_effects.push(ExternalEffect::Unknown);
                effects.deterministic = false;
                effects.may_fail = true;
            }
            TypedStatement::Configuration { .. } => {
                effects.reject(SequentialReason::Configuration);
                effects.external_effects.push(ExternalEffect::Configuration);
            }
            TypedStatement::Break { .. } => effects.reject(SequentialReason::Break),
            // Continue terminates only the current iteration; earlier journal entries survive.
            TypedStatement::Continue { .. } => {}
            TypedStatement::Expression(expression) => effects.combine(&self.expression(expression)),
        }
        effects
    }

    /// Summarizes a resolved expression, including operands of unsupported operations.
    fn expression(&mut self, expression: &TypedExpression) -> EffectSummary {
        use TypedExpressionKind::*;
        let mut effects = EffectSummary::default();
        match &expression.kind {
            Literal(_) => {}
            Variable { binding, slot, .. } => {
                effects.reads.push(access(
                    Resource::Binding(binding_resource(*binding, *slot)),
                    expression.span,
                ));
                if matches!(expression.output, Some(SemanticType::Map(_))) {
                    effects.reject(SequentialReason::MapOperation);
                }
            }
            ArrayLiteral { elements } => {
                for element in elements {
                    effects.combine(&self.expression(element));
                }
            }
            ElementAccess {
                array,
                index,
                index_extractor,
                index_dispatch,
                ..
            } => {
                effects.combine(&self.expression(index));
                self.index_callbacks(
                    index,
                    *index_extractor,
                    index_dispatch.as_deref(),
                    &mut effects,
                );
                if let Variable { binding, slot, .. } = array.kind {
                    effects.reads.push(access(
                        Resource::ArrayElement {
                            container: binding_resource(binding, slot),
                            index: self.index(index),
                        },
                        expression.span,
                    ));
                } else {
                    effects.combine(&self.expression(array));
                    effects.reject(SequentialReason::UnsupportedConstruct);
                }
                effects.may_fail = true;
            }
            Binary {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
            } => {
                effects.combine(&self.expression(left_operand));
                effects.combine(&self.expression(right_operand));
                self.binary_plan(resolution.operator, execution_plan, &mut effects);
            }
            DynamicBinary {
                dispatch,
                left_operand,
                right_operand,
                ..
            } => {
                effects.combine(&self.expression(left_operand));
                effects.combine(&self.expression(right_operand));
                for plan in dispatch.plans.iter().flatten() {
                    self.binary_plan(plan.resolution.operator, &plan.execution_plan, &mut effects);
                }
                effects.may_fail = true;
            }
            Comparison {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
            } => {
                effects.combine(&self.expression(left_operand));
                effects.combine(&self.expression(right_operand));
                self.comparison_plan(resolution.comparison, execution_plan, &mut effects);
            }
            DynamicComparison {
                dispatch,
                left_operand,
                right_operand,
                ..
            } => {
                effects.combine(&self.expression(left_operand));
                effects.combine(&self.expression(right_operand));
                for plan in dispatch.resolutions.iter().flatten() {
                    self.comparison_plan(plan.resolution.comparison, plan, &mut effects);
                }
                effects.may_fail = true;
            }
            DynamicNegation { dispatch, operand } => {
                effects.combine(&self.expression(operand));
                for plan in dispatch.plans.iter().flatten() {
                    self.binary_plan(
                        plan.binary.resolution.operator,
                        &plan.binary.execution_plan,
                        &mut effects,
                    );
                }
                effects.may_fail = true;
            }
            OpenBinary {
                left_operand,
                right_operand,
                ..
            }
            | OpenComparison {
                left_operand,
                right_operand,
                ..
            } => {
                effects.combine(&self.expression(left_operand));
                effects.combine(&self.expression(right_operand));
                unknown_operation(&mut effects);
            }
            OpenNegation { operand, .. } => {
                effects.combine(&self.expression(operand));
                unknown_operation(&mut effects);
            }
            NullCheck { operand, .. } => effects.combine(&self.expression(operand)),
            Logical {
                left_operand,
                right_operand,
                ..
            } => {
                effects.combine(&self.expression(left_operand));
                effects.combine(&self.expression(right_operand));
                self.boolean_callback(&mut effects);
            }
            LogicalNot { operand } => {
                effects.combine(&self.expression(operand));
                self.boolean_callback(&mut effects);
            }
            Call {
                function,
                arguments,
                ..
            } => {
                for argument in arguments {
                    effects.combine(&self.expression(argument));
                }
                self.call(*function, &mut effects);
            }
            Pipe {
                function,
                base,
                arguments,
            } => {
                effects.combine(&self.expression(base));
                for argument in arguments {
                    effects.combine(&self.expression(argument));
                }
                self.call(*function, &mut effects);
            }
            ArrayMethod {
                binding,
                slot,
                arguments,
                ..
            } => {
                for argument in arguments {
                    effects.combine(&self.expression(argument));
                }
                let resource = Resource::Binding(binding_resource(*binding, *slot));
                effects
                    .reads
                    .push(access(resource.clone(), expression.span));
                effects.writes.push(access(resource, expression.span));
                effects.reject(SequentialReason::UnsupportedMutation);
                effects.may_fail = true;
            }
            ElementStore {
                element,
                expression: operand,
            } => {
                effects.combine(&self.expression(operand));
                if !self.builtins.element_store(*element, operand) {
                    effects.reject(SequentialReason::UnsupportedConstruct);
                    unknown_operation(&mut effects);
                }
                effects.may_fail = true;
            }
            ArrayBoundary {
                expression: operand,
                ..
            }
            | Convert {
                expression: operand,
                ..
            }
            | DynamicConvert {
                expression: operand,
                ..
            } => {
                effects.combine(&self.expression(operand));
                // Registry base conversions and error formatters can be extension callbacks.
                effects.reject(SequentialReason::UnsupportedConstruct);
                unknown_operation(&mut effects);
            }
            MapLiteral { entries } => {
                for (key, value) in entries {
                    effects.combine(&self.expression(key));
                    effects.combine(&self.expression(value));
                }
                effects.reject(SequentialReason::MapOperation);
                effects.may_fail = true;
            }
            MapAccess { map, key, .. } => {
                effects.combine(&self.expression(map));
                effects.combine(&self.expression(key));
                effects.reject(SequentialReason::MapOperation);
                effects.may_fail = true;
            }
            SourceAs { source, .. } => {
                effects.combine(&self.expression(source));
                effects.reject(SequentialReason::UnsupportedConstruct);
            }
            RowField { row, .. } | DynamicRowField { row, .. } => {
                effects.combine(&self.expression(row));
                effects.reject(SequentialReason::UnsupportedConstruct);
                effects.may_fail = true;
            }
        }
        effects
    }

    /// Propagates declared native effects; purity alone never proves determinism.
    fn call(&self, function: crate::semantic::FunctionId, effects: &mut EffectSummary) {
        let summary = self
            .registry
            .function(function)
            .map(|descriptor| descriptor.effect_summary)
            .unwrap_or(crate::semantic::FunctionEffectSummary::UNKNOWN);
        effects.may_fail |= summary.may_fail;
        effects.deterministic &= summary.determinism == FunctionDeterminism::Deterministic;
        if !self.builtins.function_callbacks(function) {
            unknown_operation(effects);
        }
        match summary.external_effect {
            FunctionExternalEffect::None => {}
            FunctionExternalEffect::Unknown => {
                effects.external_effects.push(ExternalEffect::Unknown)
            }
            FunctionExternalEffect::ReadsExternalState => {
                effects.external_effects.push(ExternalEffect::Read)
            }
            FunctionExternalEffect::WritesExternalState => {
                effects.external_effects.push(ExternalEffect::Write)
            }
        }
        if summary.purity != FunctionPurity::Pure
            || summary.determinism != FunctionDeterminism::Deterministic
            || summary.external_effect != FunctionExternalEffect::None
        {
            effects.reject(SequentialReason::UnsafeCall(function));
        }
    }

    /// Requires actual builtin boolean evaluation rather than the source type name.
    fn boolean_callback(&self, effects: &mut EffectSummary) {
        if !self.builtins.callbacks_are_trusted() {
            unknown_operation(effects);
        }
        effects.may_fail = true;
    }

    /// Checks every executable operator in a prepared binary plan.
    fn binary_plan(
        &self,
        operator: OperatorId,
        plan: &TypedBinaryExecutionPlan,
        effects: &mut EffectSummary,
    ) {
        if !self.builtins.operator(operator) {
            unknown_operation(effects);
        }
        self.scale_plan(&plan.left_operand_scale, effects);
        self.scale_plan(&plan.right_operand_scale, effects);
        if let Some(operator) = plan.relative_adjustment_operator
            && !self.builtins.operator(operator)
        {
            unknown_operation(effects);
        }
        effects.may_fail = true;
    }

    /// Checks the resolved relation and any operators that scale its operands.
    fn comparison_plan(
        &self,
        comparison: crate::semantic::ComparisonId,
        plan: &TypedComparisonPlan,
        effects: &mut EffectSummary,
    ) {
        if !self.builtins.comparison(comparison) {
            unknown_operation(effects);
        }
        self.scale_plan(&plan.left_operand_scale, effects);
        self.scale_plan(&plan.right_operand_scale, effects);
        effects.may_fail = true;
    }

    /// Verifies the registry identity of callbacks embedded in a magnitude scale.
    fn scale_plan(&self, plan: &TypedScalePlan, effects: &mut EffectSummary) {
        for step in [&plan.numerator, &plan.denominator].into_iter().flatten() {
            if !self.builtins.operator(step.operator) {
                unknown_operation(effects);
            }
        }
    }

    /// Checks static and finite-dispatch index extraction callbacks without invoking them.
    fn index_callbacks(
        &self,
        index: &TypedExpression,
        extractor: IndexExtractor,
        dispatch: Option<&TypedIndexDispatch>,
        effects: &mut EffectSummary,
    ) {
        let trusted = match dispatch {
            Some(dispatch) => dispatch
                .domain
                .candidates
                .iter()
                .zip(&dispatch.extractors)
                .all(|(value_type, extractor)| {
                    extractor.is_none_or(|extractor| {
                        self.builtins.index_extractor(value_type.base, extractor)
                    })
                }),
            None => match &index.output {
                Some(SemanticType::Scalar(value_type)) => {
                    self.builtins.index_extractor(value_type.base, extractor)
                }
                _ => false,
            },
        };
        if !trusted {
            unknown_operation(effects);
        }
    }

    /// Builds an affine index only from verified, unscaled signed-integer operations.
    fn index(&self, expression: &TypedExpression) -> IndexAccess {
        match self.affine(expression) {
            Some((0, offset)) => IndexAccess::Constant(offset),
            Some((coefficient, offset)) => {
                self.iteration.map_or(IndexAccess::Unknown, |iteration| {
                    IndexAccess::Affine(AffineIndex {
                        iteration,
                        coefficient,
                        offset,
                    })
                })
            }
            None => IndexAccess::Unknown,
        }
    }

    /// Recursively folds exact integer addition/subtraction and constant multiplication.
    fn affine(&self, expression: &TypedExpression) -> Option<(i128, i128)> {
        match &expression.kind {
            TypedExpressionKind::Literal(value) => {
                self.builtins.integer_literal(value).map(|value| (0, value))
            }
            TypedExpressionKind::Variable { binding, slot, .. }
                if self.iteration == Some(binding_resource(*binding, *slot)) =>
            {
                let Some(SemanticType::Scalar(value_type)) = expression.output else {
                    return None;
                };
                self.builtins.signed_integer(value_type).then_some((1, 0))
            }
            TypedExpressionKind::Binary {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
            } if self.builtins.operator(resolution.operator)
                && self.builtins.signed_integer(resolution.output)
                && resolution.left_operand_scale.is_identity()
                && resolution.right_operand_scale.is_identity()
                && resolution.relative_adjustment.is_none()
                && execution_plan.left_operand_scale.is_identity()
                && execution_plan.right_operand_scale.is_identity()
                && execution_plan.relative_adjustment_operator.is_none()
                && self.signed_expression(left_operand)
                && self.signed_expression(right_operand) =>
            {
                let left = self.affine(left_operand)?;
                let right = self.affine(right_operand)?;
                affine_operation(
                    self.registry.operator(resolution.operator).ok()?.operator,
                    left,
                    right,
                )
            }
            TypedExpressionKind::DynamicBinary {
                operator,
                dispatch,
                left_operand,
                right_operand,
            } if self.signed_expression(left_operand)
                && self.signed_expression(right_operand)
                && !dispatch.plans.is_empty()
                && dispatch.plans.iter().flatten().all(|plan| {
                    self.builtins.operator(plan.resolution.operator)
                        && self
                            .registry
                            .operator(plan.resolution.operator)
                            .is_ok_and(|descriptor| descriptor.operator == *operator)
                        && self.builtins.signed_integer(plan.resolution.output)
                        && plan.resolution.left_operand_scale.is_identity()
                        && plan.resolution.right_operand_scale.is_identity()
                        && plan.resolution.relative_adjustment.is_none()
                        && plan.execution_plan.left_operand_scale.is_identity()
                        && plan.execution_plan.right_operand_scale.is_identity()
                        && plan.execution_plan.relative_adjustment_operator.is_none()
                }) =>
            {
                affine_operation(
                    *operator,
                    self.affine(left_operand)?,
                    self.affine(right_operand)?,
                )
            }
            _ => None,
        }
    }

    /// Requires a statically plain signed integer representation for index algebra.
    fn signed_expression(&self, expression: &TypedExpression) -> bool {
        if let Some(domain) = expression.complete_type_domain() {
            return !domain.is_empty()
                && domain
                    .candidates
                    .iter()
                    .all(|value_type| self.builtins.signed_integer(*value_type));
        }
        matches!(&expression.output, Some(SemanticType::Scalar(value_type)) if self.builtins.signed_integer(*value_type))
    }
}

/// Combines two proven affine values with exact checked coefficient arithmetic.
fn affine_operation(
    operator: BinaryOperator,
    left: (i128, i128),
    right: (i128, i128),
) -> Option<(i128, i128)> {
    match operator {
        BinaryOperator::Addition => {
            Some((left.0.checked_add(right.0)?, left.1.checked_add(right.1)?))
        }
        BinaryOperator::Subtraction => {
            Some((left.0.checked_sub(right.0)?, left.1.checked_sub(right.1)?))
        }
        BinaryOperator::Multiplication if left.0 == 0 => {
            Some((right.0.checked_mul(left.1)?, right.1.checked_mul(left.1)?))
        }
        BinaryOperator::Multiplication if right.0 == 0 => {
            Some((left.0.checked_mul(right.1)?, left.1.checked_mul(right.1)?))
        }
        _ => None,
    }
}

/// Creates a resolved identity from the binding and its allocated runtime slot.
fn binding_resource(binding: BindingId, slot: LocalVariableSlot) -> BindingResource {
    BindingResource { binding, slot }
}

/// Creates one located access for explanations and later optimization passes.
fn access(resource: Resource, span: Span) -> ResourceAccess {
    ResourceAccess { resource, span }
}

/// Marks unverified extension dispatch as potentially effectful and nondeterministic.
fn unknown_operation(effects: &mut EffectSummary) {
    effects.reject(SequentialReason::UnknownOperation);
    if !effects.external_effects.contains(&ExternalEffect::Unknown) {
        effects.external_effects.push(ExternalEffect::Unknown);
    }
    effects.deterministic = false;
    effects.may_fail = true;
}

/// Collects all slots declared within a region, including branch and loop locals.
fn owned_slots(statements: &[TypedStatement]) -> HashSet<LocalVariableSlot> {
    let mut slots = HashSet::new();
    for statement in statements {
        match statement {
            TypedStatement::VariableDeclaration { slot, .. } => {
                slots.insert(*slot);
            }
            TypedStatement::Block(body) | TypedStatement::While { body, .. } => {
                slots.extend(owned_slots(&body.statements));
            }
            TypedStatement::If {
                body, else_body, ..
            } => {
                slots.extend(owned_slots(&body.statements));
                if let Some(body) = else_body {
                    slots.extend(owned_slots(&body.statements));
                }
            }
            TypedStatement::For { slot, body, .. } | TypedStatement::ForEach { slot, body, .. } => {
                slots.insert(*slot);
                slots.extend(owned_slots(&body.statements));
            }
            _ => {}
        }
    }
    slots
}

/// Proves disjointness only for the same nonconstant affine index on one slot.
fn disjoint_iterations(left: &Resource, right: &Resource, iteration: BindingResource) -> bool {
    match (left, right) {
        (
            Resource::ArrayElement {
                index: IndexAccess::Affine(left),
                ..
            },
            Resource::ArrayElement {
                index: IndexAccess::Affine(right),
                ..
            },
        ) => left == right && left.iteration == iteration && left.coefficient != 0,
        _ => false,
    }
}

/// Builds loop-carried evidence and the worker snapshot/journal slot lists.
fn classify_loop(
    span: Span,
    effects: EffectSummary,
    iteration: BindingResource,
    local_slots: &HashSet<LocalVariableSlot>,
) -> LoopAnalysis {
    let mut reasons = effects.reasons.clone();
    let mut dependencies = Vec::new();
    let mut written_array_slots = Vec::new();
    for write in &effects.writes {
        let slot = write.resource.slot();
        if local_slots.contains(&slot) {
            if matches!(&write.resource, Resource::Binding(binding) if *binding == iteration) {
                push_reason(&mut reasons, SequentialReason::CapturedMutation(iteration));
            }
            if let Resource::ArrayElement { container, .. } = write.resource {
                push_reason(
                    &mut reasons,
                    SequentialReason::LocalArrayMutation(container),
                );
            }
            continue;
        }
        match &write.resource {
            Resource::Binding(binding) => {
                push_reason(&mut reasons, SequentialReason::CapturedMutation(*binding))
            }
            Resource::Map { .. } => push_reason(&mut reasons, SequentialReason::MapOperation),
            Resource::ArrayElement { container, index } => {
                written_array_slots.push(slot);
                if !matches!(index, IndexAccess::Affine(index) if index.iteration == iteration && index.coefficient != 0)
                {
                    push_reason(&mut reasons, SequentialReason::NonAffineWrite(*container));
                }
            }
        }
        for read in &effects.reads {
            if read.resource.slot() != slot {
                continue;
            }
            if matches!(read.resource, Resource::Binding(_))
                && matches!(write.resource, Resource::ArrayElement { .. })
            {
                let Resource::ArrayElement { container, .. } = write.resource else {
                    unreachable!();
                };
                push_reason(
                    &mut reasons,
                    SequentialReason::WholeContainerRead(container),
                );
            }
            if !disjoint_iterations(&write.resource, &read.resource, iteration) {
                dependencies.push(dependency(DependencyKind::ReadAfterWrite, write, read));
                dependencies.push(dependency(DependencyKind::WriteAfterRead, read, write));
            }
        }
        for other in &effects.writes {
            if other.resource.slot() == slot
                && !disjoint_iterations(&write.resource, &other.resource, iteration)
            {
                dependencies.push(dependency(DependencyKind::WriteAfterWrite, write, other));
            }
        }
    }
    if !dependencies.is_empty() {
        push_reason(&mut reasons, SequentialReason::ConflictingAccesses);
    }
    if !effects.external_effects.is_empty() || !effects.deterministic {
        // Every known source normally contributes a more precise reason too.
        if reasons.is_empty() {
            reasons.push(SequentialReason::UnknownOperation);
        }
    }
    sort_slots(&mut written_array_slots);
    let mut capture_read_slots: Vec<_> = effects
        .reads
        .iter()
        .map(|read| read.resource.slot())
        .filter(|slot| !local_slots.contains(slot))
        .collect();
    // A write target's length, element contract, and baseline are part of the snapshot.
    capture_read_slots.extend(&written_array_slots);
    sort_slots(&mut capture_read_slots);
    let parallelism = if reasons.is_empty() {
        Parallelism::IndependentIterations {
            written_array_slots,
            capture_read_slots,
        }
    } else {
        Parallelism::Sequential { reasons }
    };
    LoopAnalysis {
        span,
        cost_per_iteration: WorkCost::default(),
        effects,
        parallelism,
        dependencies,
    }
}

/// Creates structured access-pair evidence for a future reduction/dependence pass.
fn dependency(
    kind: DependencyKind,
    source: &ResourceAccess,
    destination: &ResourceAccess,
) -> LoopCarriedDependency {
    LoopCarriedDependency {
        kind,
        source: source.clone(),
        destination: destination.clone(),
        reason: SequentialReason::ConflictingAccesses,
    }
}

/// Retains each rejection once without relying on debug-string diagnostics.
fn push_reason(reasons: &mut Vec<SequentialReason>, reason: SequentialReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

/// Produces stable, duplicate-free slot lists for runtime preparation.
fn sort_slots(slots: &mut Vec<LocalVariableSlot>) {
    slots.sort_unstable_by_key(|slot| slot.0);
    slots.dedup();
}

#[cfg(test)]
#[path = "resolved.tests.rs"]
mod tests;
