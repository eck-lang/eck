use std::collections::{HashMap, HashSet};

use super::Runtime;
use crate::RuntimeError;
use ir::{
    LocalVariableSlot, TypedBinaryExecutionPlan, TypedBlock, TypedExpression, TypedExpressionKind,
    TypedStatement,
};
use language_core::{BinaryOperatorDescriptor, ComparisonExecutor, ResolvedBinaryOperator, Value};

/// Stores a linear execution plan for the hot portion of one range-loop body.
///
/// Source semantics remain in the typed AST. This plan only represents
/// statically plain binary expressions whose resolved operators cannot promote
/// at runtime; every other statement remains an AST step.
pub(super) struct LoopBodyExecutionPlan<'program> {
    steps: Vec<LoopBodyExecutionStep<'program>>,
    pub(super) value_stack_capacity: usize,
}

/// Represents either direct numeric instructions or one fallback AST statement.
enum LoopBodyExecutionStep<'program> {
    DirectInstructions(Vec<DirectLoopInstruction<'program>>),
    DirectComparisonConditional {
        comparison: ComparisonExecutor,
        left_operand: DirectConditionalOperand<'program>,
        right_operand: DirectConditionalOperand<'program>,
        body: &'program TypedBlock,
    },
    Statement(&'program TypedStatement),
}

/// Represents one directly readable operand in a comparison-only conditional.
enum DirectConditionalOperand<'program> {
    Literal(&'program Value),
    Local(LocalVariableSlot),
}

/// Represents one stack-machine instruction for a direct loop-body expression.
enum DirectLoopInstruction<'program> {
    LoadLiteral(&'program Value),
    LoadLocal {
        slot: LocalVariableSlot,
        consume: bool,
    },
    ExecuteBinary {
        descriptor: BinaryOperatorDescriptor,
        adjustment_descriptor: Option<BinaryOperatorDescriptor>,
        resolution: ResolvedBinaryOperator,
        execution_plan: &'program TypedBinaryExecutionPlan,
        skip_initial_configuration_transform: bool,
    },
    StoreLocal(LocalVariableSlot),
}

impl<'registry> Runtime<'registry> {
    /// Compiles direct instructions for eligible contiguous declarations in a loop body.
    pub(super) fn compile_loop_body_execution_plan<'program>(
        &self,
        range_slot: LocalVariableSlot,
        body: &'program TypedBlock,
    ) -> Result<LoopBodyExecutionPlan<'program>, RuntimeError> {
        let mut steps = Vec::new();
        let mut direct_instructions = Vec::new();
        let mut value_stack_depth = 0;
        let mut value_stack_capacity = 0;
        let mut remaining_local_uses = self.count_loop_body_local_uses(body);
        let mut reinitialized_slots = HashSet::from([range_slot]);

        for statement in &body.statements {
            if let Some(conditional) = self.compile_direct_comparison_conditional(statement)? {
                if !direct_instructions.is_empty() {
                    steps.push(LoopBodyExecutionStep::DirectInstructions(
                        direct_instructions,
                    ));
                    direct_instructions = Vec::new();
                }
                steps.push(conditional);
                continue;
            }
            let Some(instructions) = self.compile_direct_declaration(
                statement,
                &mut remaining_local_uses,
                &reinitialized_slots,
            )?
            else {
                if !direct_instructions.is_empty() {
                    steps.push(LoopBodyExecutionStep::DirectInstructions(
                        direct_instructions,
                    ));
                    direct_instructions = Vec::new();
                }
                steps.push(LoopBodyExecutionStep::Statement(statement));
                continue;
            };
            if let TypedStatement::VariableDeclaration { slot, .. } = statement {
                reinitialized_slots.insert(*slot);
            }

            for instruction in instructions {
                match instruction {
                    DirectLoopInstruction::LoadLiteral(_)
                    | DirectLoopInstruction::LoadLocal { .. } => {
                        value_stack_depth += 1;
                        value_stack_capacity = value_stack_capacity.max(value_stack_depth);
                    }
                    DirectLoopInstruction::ExecuteBinary { .. } => value_stack_depth -= 1,
                    DirectLoopInstruction::StoreLocal(_) => value_stack_depth -= 1,
                }
                direct_instructions.push(instruction);
            }
        }
        if !direct_instructions.is_empty() {
            steps.push(LoopBodyExecutionStep::DirectInstructions(
                direct_instructions,
            ));
        }

        Ok(LoopBodyExecutionPlan {
            steps,
            value_stack_capacity,
        })
    }

    /// Lowers one statically plain variable declaration into direct stack instructions.
    fn compile_direct_declaration<'program>(
        &self,
        statement: &'program TypedStatement,
        remaining_local_uses: &mut HashMap<LocalVariableSlot, usize>,
        reinitialized_slots: &HashSet<LocalVariableSlot>,
    ) -> Result<Option<Vec<DirectLoopInstruction<'program>>>, RuntimeError> {
        let TypedStatement::VariableDeclaration {
            slot, expression, ..
        } = statement
        else {
            return Ok(None);
        };
        let mut instructions = Vec::new();
        let initial_remaining_local_uses = remaining_local_uses.clone();
        if !self.append_direct_expression(
            expression,
            &mut instructions,
            remaining_local_uses,
            reinitialized_slots,
        )? {
            *remaining_local_uses = initial_remaining_local_uses;
            return Ok(None);
        }
        instructions.push(DirectLoopInstruction::StoreLocal(*slot));
        Ok(Some(instructions))
    }

    /// Lowers a plain comparison conditional while retaining its normal body execution.
    fn compile_direct_comparison_conditional<'program>(
        &self,
        statement: &'program TypedStatement,
    ) -> Result<Option<LoopBodyExecutionStep<'program>>, RuntimeError> {
        let TypedStatement::If {
            condition,
            body,
            else_body,
            ..
        } = statement
        else {
            return Ok(None);
        };
        if else_body.is_some() {
            return Ok(None);
        }
        let TypedExpressionKind::Comparison {
            resolution,
            left_operand,
            right_operand,
        } = &condition.kind
        else {
            return Ok(None);
        };
        if !resolution.left_operand_scale.is_identity()
            || !resolution.right_operand_scale.is_identity()
        {
            return Ok(None);
        }
        let descriptor = self.registry.comparison(resolution.comparison)?;
        let Some(left_operand) = Self::direct_conditional_operand(left_operand) else {
            return Ok(None);
        };
        let Some(right_operand) = Self::direct_conditional_operand(right_operand) else {
            return Ok(None);
        };
        Ok(Some(LoopBodyExecutionStep::DirectComparisonConditional {
            comparison: descriptor.execute,
            left_operand,
            right_operand,
            body,
        }))
    }

    /// Converts a literal or local reference into one direct conditional operand.
    fn direct_conditional_operand(
        expression: &'_ TypedExpression,
    ) -> Option<DirectConditionalOperand<'_>> {
        match &expression.kind {
            TypedExpressionKind::Literal(value) => Some(DirectConditionalOperand::Literal(value)),
            TypedExpressionKind::Variable { slot, .. } => {
                Some(DirectConditionalOperand::Local(*slot))
            }
            TypedExpressionKind::Binary { .. }
            | TypedExpressionKind::Comparison { .. }
            | TypedExpressionKind::NullCheck { .. }
            | TypedExpressionKind::Logical { .. }
            | TypedExpressionKind::LogicalNot { .. }
            | TypedExpressionKind::Convert { .. }
            | TypedExpressionKind::Call { .. }
            | TypedExpressionKind::ArrayLiteral { .. }
            | TypedExpressionKind::ElementStore { .. }
            | TypedExpressionKind::ElementAccess { .. }
            | TypedExpressionKind::DynamicBinary { .. }
            | TypedExpressionKind::DynamicComparison { .. }
            | TypedExpressionKind::DynamicConvert { .. }
            | TypedExpressionKind::Pipe { .. } => None,
        }
    }

    /// Appends direct instructions when an expression has fixed, plain binary dispatch.
    fn append_direct_expression<'program>(
        &self,
        expression: &'program TypedExpression,
        instructions: &mut Vec<DirectLoopInstruction<'program>>,
        remaining_local_uses: &mut HashMap<LocalVariableSlot, usize>,
        reinitialized_slots: &HashSet<LocalVariableSlot>,
    ) -> Result<bool, RuntimeError> {
        match &expression.kind {
            TypedExpressionKind::Literal(value) => {
                instructions.push(DirectLoopInstruction::LoadLiteral(value));
                Ok(true)
            }
            TypedExpressionKind::Variable { slot, .. } => {
                let remaining_uses = remaining_local_uses
                    .get_mut(slot)
                    .expect("typed loop reference has a recorded local use");
                *remaining_uses -= 1;
                instructions.push(DirectLoopInstruction::LoadLocal {
                    slot: *slot,
                    consume: *remaining_uses == 0 && reinitialized_slots.contains(slot),
                });
                Ok(true)
            }
            TypedExpressionKind::Binary {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
            } => {
                let descriptor = self.registry.operator(resolution.operator)?;
                let adjustment_is_context_free = execution_plan
                    .relative_adjustment_operator
                    .map(|operator| self.registry.operator(operator))
                    .transpose()?
                    .is_none_or(|descriptor| descriptor.context_execute.is_none());
                let plain_binary_execution = execution_plan.relative_adjustment_operator.is_none()
                    && execution_plan.left_operand_scale.is_identity()
                    && execution_plan.right_operand_scale.is_identity()
                    && resolution.output.subtype.is_none();
                let context_free_execution = descriptor.context_execute.is_none()
                    && !self.scale_plan_uses_context(&execution_plan.left_operand_scale)?
                    && !self.scale_plan_uses_context(&execution_plan.right_operand_scale)?
                    && adjustment_is_context_free;
                let direct_execution = (plain_binary_execution || context_free_execution)
                    && left_operand.output.is_some()
                    && right_operand.output.is_some();
                if !direct_execution {
                    return Ok(false);
                }

                let instruction_start = instructions.len();
                if !self.append_direct_expression(
                    left_operand,
                    instructions,
                    remaining_local_uses,
                    reinitialized_slots,
                )? || !self.append_direct_expression(
                    right_operand,
                    instructions,
                    remaining_local_uses,
                    reinitialized_slots,
                )? {
                    instructions.truncate(instruction_start);
                    return Ok(false);
                }
                let skip_initial_configuration_transform = self.configuration.uses_initial_values()
                    && self
                        .registry
                        .initial_result_transform_is_identity(descriptor.result_type)?;
                instructions.push(DirectLoopInstruction::ExecuteBinary {
                    descriptor: descriptor.clone(),
                    adjustment_descriptor: execution_plan
                        .relative_adjustment_operator
                        .map(|operator| self.registry.operator(operator).cloned())
                        .transpose()?,
                    resolution: *resolution,
                    execution_plan,
                    skip_initial_configuration_transform,
                });
                Ok(true)
            }
            TypedExpressionKind::Comparison { .. }
            | TypedExpressionKind::NullCheck { .. }
            | TypedExpressionKind::Logical { .. }
            | TypedExpressionKind::LogicalNot { .. }
            | TypedExpressionKind::Convert { .. }
            | TypedExpressionKind::Call { .. }
            | TypedExpressionKind::ArrayLiteral { .. }
            | TypedExpressionKind::ElementStore { .. }
            | TypedExpressionKind::ElementAccess { .. }
            | TypedExpressionKind::DynamicBinary { .. }
            | TypedExpressionKind::DynamicComparison { .. }
            | TypedExpressionKind::DynamicConvert { .. }
            | TypedExpressionKind::Pipe { .. } => Ok(false),
        }
    }

    /// Counts every local-variable read in a loop body before direct lowering begins.
    fn count_loop_body_local_uses(&self, body: &TypedBlock) -> HashMap<LocalVariableSlot, usize> {
        let mut local_uses = HashMap::new();
        for statement in &body.statements {
            Self::count_statement_local_uses(statement, &mut local_uses);
        }
        local_uses
    }

    /// Counts local reads inside one statement and all nested expressions.
    fn count_statement_local_uses(
        statement: &TypedStatement,
        local_uses: &mut HashMap<LocalVariableSlot, usize>,
    ) {
        match statement {
            TypedStatement::VariableDeclaration { expression, .. }
            | TypedStatement::Assignment { expression, .. }
            | TypedStatement::Expression(expression) => {
                Self::count_expression_local_uses(expression, local_uses);
            }
            TypedStatement::If {
                condition,
                body,
                else_body,
                ..
            } => {
                Self::count_expression_local_uses(condition, local_uses);
                for nested_statement in &body.statements {
                    Self::count_statement_local_uses(nested_statement, local_uses);
                }
                if let Some(else_body) = else_body {
                    for nested_statement in &else_body.statements {
                        Self::count_statement_local_uses(nested_statement, local_uses);
                    }
                }
            }
            TypedStatement::For {
                start, end, body, ..
            } => {
                Self::count_expression_local_uses(start, local_uses);
                Self::count_expression_local_uses(end, local_uses);
                for nested_statement in &body.statements {
                    Self::count_statement_local_uses(nested_statement, local_uses);
                }
            }
            TypedStatement::While {
                condition, body, ..
            } => {
                Self::count_expression_local_uses(condition, local_uses);
                for nested_statement in &body.statements {
                    Self::count_statement_local_uses(nested_statement, local_uses);
                }
            }
            TypedStatement::Block(body) => {
                for nested_statement in &body.statements {
                    Self::count_statement_local_uses(nested_statement, local_uses);
                }
            }
            TypedStatement::Configuration { .. } => {}
            TypedStatement::IndexedAssignment {
                index, expression, ..
            } => {
                Self::count_expression_local_uses(index, local_uses);
                Self::count_expression_local_uses(expression, local_uses);
            }
            TypedStatement::Break { .. } | TypedStatement::Continue { .. } => {}
        }
    }

    /// Counts local reads inside a typed expression tree.
    fn count_expression_local_uses(
        expression: &TypedExpression,
        local_uses: &mut HashMap<LocalVariableSlot, usize>,
    ) {
        match &expression.kind {
            TypedExpressionKind::Literal(_) => {}
            TypedExpressionKind::Variable { slot, .. } => {
                *local_uses.entry(*slot).or_default() += 1;
            }
            TypedExpressionKind::Binary {
                left_operand,
                right_operand,
                ..
            }
            | TypedExpressionKind::Comparison {
                left_operand,
                right_operand,
                ..
            }
            | TypedExpressionKind::Logical {
                left_operand,
                right_operand,
                ..
            }
            | TypedExpressionKind::DynamicBinary {
                left_operand,
                right_operand,
                ..
            }
            | TypedExpressionKind::DynamicComparison {
                left_operand,
                right_operand,
                ..
            } => {
                Self::count_expression_local_uses(left_operand, local_uses);
                Self::count_expression_local_uses(right_operand, local_uses);
            }
            TypedExpressionKind::Convert { expression, .. }
            | TypedExpressionKind::DynamicConvert { expression, .. } => {
                Self::count_expression_local_uses(expression, local_uses);
            }
            TypedExpressionKind::LogicalNot { operand } => {
                Self::count_expression_local_uses(operand, local_uses);
            }
            TypedExpressionKind::NullCheck { operand, .. } => {
                Self::count_expression_local_uses(operand, local_uses);
            }
            TypedExpressionKind::Call { arguments, .. } => {
                for argument in arguments {
                    Self::count_expression_local_uses(argument, local_uses);
                }
            }
            TypedExpressionKind::Pipe {
                base, arguments, ..
            } => {
                Self::count_expression_local_uses(base, local_uses);
                for argument in arguments {
                    Self::count_expression_local_uses(argument, local_uses);
                }
            }
            TypedExpressionKind::ArrayLiteral { elements, .. } => {
                for element in elements {
                    Self::count_expression_local_uses(element, local_uses);
                }
            }
            TypedExpressionKind::ElementStore { expression, .. } => {
                Self::count_expression_local_uses(expression, local_uses);
            }
            TypedExpressionKind::ElementAccess { array, index, .. } => {
                Self::count_expression_local_uses(array, local_uses);
                Self::count_expression_local_uses(index, local_uses);
            }
        }
    }

    /// Executes a loop body through direct instructions and AST fallback steps.
    pub(super) fn execute_loop_body(
        &mut self,
        plan: &LoopBodyExecutionPlan<'_>,
        value_stack: &mut Vec<Value>,
    ) -> Result<(), RuntimeError> {
        for step in &plan.steps {
            match step {
                LoopBodyExecutionStep::DirectInstructions(instructions) => {
                    self.execute_direct_loop_instructions(instructions, value_stack)?;
                }
                LoopBodyExecutionStep::DirectComparisonConditional {
                    comparison,
                    left_operand,
                    right_operand,
                    body,
                } => {
                    let left_operand = self.read_direct_conditional_operand(left_operand);
                    let right_operand = self.read_direct_conditional_operand(right_operand);
                    if comparison(&left_operand, &right_operand)? {
                        self.execute_block(body)?;
                    }
                }
                LoopBodyExecutionStep::Statement(statement) => self.execute_statement(statement)?,
            }
            if self.loop_control.is_some() {
                break;
            }
        }
        Ok(())
    }

    /// Executes direct instructions using a reusable stack of runtime values.
    fn execute_direct_loop_instructions(
        &mut self,
        instructions: &[DirectLoopInstruction<'_>],
        value_stack: &mut Vec<Value>,
    ) -> Result<(), RuntimeError> {
        value_stack.clear();
        for instruction in instructions {
            match instruction {
                DirectLoopInstruction::LoadLiteral(value) => value_stack.push((*value).clone()),
                DirectLoopInstruction::LoadLocal { slot, consume } => {
                    let value = if *consume {
                        self.local_values[slot.0]
                            .take()
                            .expect("compiled loop local is initialized before use")
                    } else {
                        self.local_values[slot.0]
                            .clone()
                            .expect("compiled loop local is initialized before use")
                    };
                    value_stack.push(value);
                }
                DirectLoopInstruction::ExecuteBinary {
                    descriptor,
                    adjustment_descriptor,
                    resolution,
                    execution_plan,
                    skip_initial_configuration_transform,
                } => {
                    let right_operand = value_stack
                        .pop()
                        .expect("direct binary instruction has a right operand");
                    let mut left_operand = value_stack
                        .pop()
                        .expect("direct binary instruction has a left operand");
                    let plain_direct_execution =
                        execution_plan.relative_adjustment_operator.is_none()
                            && execution_plan.left_operand_scale.is_identity()
                            && execution_plan.right_operand_scale.is_identity()
                            && resolution.output.subtype.is_none();
                    let descriptor = if plain_direct_execution {
                        self.redispatch_for_dynamic_types(descriptor, &left_operand, &right_operand)
                    } else {
                        descriptor
                    };
                    let value = if plain_direct_execution && left_operand.is_uniquely_owned() {
                        if let Some(execute) = descriptor.in_place_execute {
                            execute(&mut left_operand, &right_operand)?;
                            left_operand
                        } else {
                            self.execute_binary_operator(descriptor, &left_operand, &right_operand)?
                        }
                    } else if plain_direct_execution {
                        let value = self.execute_binary_operator(
                            descriptor,
                            &left_operand,
                            &right_operand,
                        )?;
                        value
                    } else {
                        self.execute_binary_plan_owned(
                            resolution,
                            execution_plan,
                            descriptor,
                            adjustment_descriptor.as_ref(),
                            left_operand,
                            right_operand,
                        )?
                    }
                    .with_subtype(resolution.output.subtype);
                    let value = if *skip_initial_configuration_transform {
                        value
                    } else {
                        self.registry
                            .transform_owned_configured_result(value, &self.configuration)?
                    };
                    value_stack.push(value);
                }
                DirectLoopInstruction::StoreLocal(slot) => {
                    let value = value_stack
                        .pop()
                        .expect("direct declaration produces one value");
                    self.store_local_value(*slot, value);
                }
            }
        }
        debug_assert!(value_stack.is_empty());
        Ok(())
    }

    /// Reads one direct conditional operand without allocating an expression result.
    fn read_direct_conditional_operand(&self, operand: &DirectConditionalOperand<'_>) -> Value {
        match operand {
            DirectConditionalOperand::Literal(value) => (*value).clone(),
            DirectConditionalOperand::Local(slot) => self.local_values[slot.0]
                .clone()
                .expect("compiled conditional local is initialized before use"),
        }
    }
}
