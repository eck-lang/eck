//! Compilation of scalar operations whose complete value type is dynamic.
//!
//! These helpers build dispatch plans over finite [`ValueType`] domains. They
//! are shared by ordinary scalar expressions and array elements whose stored
//! subtype or adaptive integer representation is only known at runtime.

use std::sync::Arc;

use crate::semantic::{
    BinaryOperator as CoreBinaryOperator, ComparisonOperator as CoreComparisonOperator, CoreError,
    ResolvedSubtypeConversion, Scale, SemanticType, SubtypeId, TypeId, ValueType,
};

use crate::CompileError;
use crate::ir::{CompleteTypeDomain, TypedExpression, TypedExpressionKind, TypedUnaryNegationPlan};

use super::Compiler;

impl Compiler<'_> {
    /// Reports whether a base belongs to the adaptive signed integer family.
    pub(crate) fn is_signed_integer_base(&self, base: TypeId) -> bool {
        self.registry
            .signed_integer_widening_types()
            .contains(&base)
    }

    /// Returns a domain for a scalar that is exact unless the expression has
    /// already recorded a broader complete-type contract.
    pub(super) fn scalar_complete_type_domain(
        &self,
        expression: &TypedExpression,
        representative: ValueType,
    ) -> Arc<CompleteTypeDomain> {
        expression
            .complete_type_domain()
            .unwrap_or_else(|| CompleteTypeDomain::from_candidates(self.registry, [representative]))
    }

    /// Returns the signed integer result identities a context-aware operation
    /// may produce after its statically described result.
    fn operation_result_candidates(&self, output: ValueType, may_promote: bool) -> Vec<ValueType> {
        if !may_promote || !self.is_signed_integer_base(output.base) {
            return vec![output];
        }
        let signed_types = self.registry.signed_integer_widening_types();
        let Some(position) = signed_types.iter().position(|base| *base == output.base) else {
            return vec![output];
        };
        signed_types
            .into_iter()
            .skip(position)
            .map(|base| ValueType {
                base,
                subtype: output.subtype,
            })
            .collect()
    }

    /// Builds a result domain when a binary plan contains value-dependent
    /// integer promotion or produces differing complete identities.
    pub(super) fn binary_result_domain(
        &self,
        resolution: &crate::semantic::ResolvedBinaryOperator,
        execution_plan: &crate::ir::TypedBinaryExecutionPlan,
    ) -> Option<Arc<CompleteTypeDomain>> {
        let may_promote = self
            .registry
            .operator(resolution.operator)
            .ok()
            .is_some_and(|descriptor| descriptor.context_execute.is_some())
            || self.scale_plan_uses_context(&execution_plan.left_operand_scale)
            || self.scale_plan_uses_context(&execution_plan.right_operand_scale);
        let candidates = self.operation_result_candidates(resolution.output, may_promote);
        (candidates.len() > 1)
            .then(|| CompleteTypeDomain::from_candidates(self.registry, candidates))
    }

    /// Reports whether a pre-resolved scale contains a context-aware operator.
    fn scale_plan_uses_context(&self, scale_plan: &crate::ir::TypedScalePlan) -> bool {
        [
            scale_plan.numerator.as_ref(),
            scale_plan.denominator.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(|step| {
            self.registry
                .operator(step.operator)
                .ok()
                .is_some_and(|descriptor| descriptor.context_execute.is_some())
        })
    }

    /// Compiles one binary operation whose operand complete types are dynamic.
    ///
    /// The compiler resolves the operation once per candidate complete type and
    /// stores the plans in a dispatch table. The runtime selects a plan from
    /// the values' complete types without resolving subtype rules again.
    pub(super) fn dynamic_binary(
        &self,
        operator: CoreBinaryOperator,
        left_operand: TypedExpression,
        left_domain: Arc<CompleteTypeDomain>,
        right_operand: TypedExpression,
        right_domain: Arc<CompleteTypeDomain>,
        span: crate::syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let mut plans = Vec::with_capacity(left_domain.len() * right_domain.len());
        let mut first_error: Option<CompileError> = None;
        let mut reference_output: Option<ValueType> = None;
        let mut result_candidates = Vec::new();
        for candidate_left in left_domain.candidates.iter().copied() {
            for candidate_right in right_domain.candidates.iter().copied() {
                let plan =
                    match self.resolve_binary_plan(operator, candidate_left, candidate_right, span)
                    {
                        Ok(plan) => {
                            reference_output.get_or_insert(plan.resolution.output);
                            result_candidates.push(plan.resolution.output);
                            if let Some(domain) = &plan.execution_plan.result_domain {
                                result_candidates.extend(domain.candidates.iter().copied());
                            }
                            Some(plan)
                        }
                        Err(error) => {
                            first_error.get_or_insert(error);
                            None
                        }
                    };
                plans.push(plan);
            }
        }
        if plans.iter().all(|plan| plan.is_none()) {
            return Err(first_error.unwrap_or_else(|| {
                CompileError::new(
                    span,
                    format!("operator `{operator}` is not defined for the operand types"),
                )
            }));
        }
        let result_domain = CompleteTypeDomain::from_candidates(self.registry, result_candidates);
        let result_domain = (result_domain.len() > 1).then_some(result_domain);
        let output = reference_output.ok_or_else(|| {
            CompileError::new(
                span,
                format!("operator `{operator}` is not defined for the operand types"),
            )
        })?;
        Ok(TypedExpression {
            output: Some(SemanticType::Scalar(output)),
            kind: TypedExpressionKind::DynamicBinary {
                operator,
                dispatch: Box::new(crate::ir::TypedBinaryDispatch {
                    left_domain,
                    right_domain,
                    plans,
                    result_domain,
                }),
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
            },
            span,
        })
    }

    /// Compiles one comparison whose operand complete types are dynamic.
    ///
    /// The runtime selects the relation from the operands' complete types
    /// instead of resolving one comparison type at compile time.
    pub(super) fn dynamic_comparison(
        &self,
        operator: CoreComparisonOperator,
        left_operand: TypedExpression,
        left_domain: Arc<CompleteTypeDomain>,
        right_operand: TypedExpression,
        right_domain: Arc<CompleteTypeDomain>,
        span: crate::syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let mut resolutions = Vec::with_capacity(left_domain.len() * right_domain.len());
        let mut first_error: Option<CompileError> = None;
        let mut output: Option<ValueType> = None;
        for candidate_left in left_domain.candidates.iter().copied() {
            for candidate_right in right_domain.candidates.iter().copied() {
                let resolution = match self.registry.resolve_comparison_operation(
                    operator,
                    candidate_left,
                    candidate_right,
                ) {
                    Ok(resolution) => {
                        let left_operand_scale = self
                            .compile_scale_plan(
                                candidate_left.base,
                                resolution.left_operand_scale,
                                span,
                            )
                            .map(|plan| plan.0);
                        let right_operand_scale = left_operand_scale.and_then(|left| {
                            self.compile_scale_plan(
                                candidate_right.base,
                                resolution.right_operand_scale,
                                span,
                            )
                            .map(|plan| (left, plan.0))
                        });
                        match right_operand_scale {
                            Ok((left_operand_scale, right_operand_scale)) => {
                                output.get_or_insert(resolution.output);
                                Some(crate::ir::TypedComparisonPlan {
                                    resolution,
                                    left_operand_scale,
                                    right_operand_scale,
                                })
                            }
                            Err(error) => {
                                first_error.get_or_insert(error);
                                None
                            }
                        }
                    }
                    Err(error) => {
                        first_error.get_or_insert(CompileError::core(span, error));
                        None
                    }
                };
                resolutions.push(resolution);
            }
        }
        if resolutions.iter().all(|resolution| resolution.is_none()) {
            return Err(first_error.unwrap_or_else(|| {
                CompileError::new(
                    span,
                    format!("comparison `{operator}` is not defined for the operand types"),
                )
            }));
        }
        Ok(TypedExpression {
            output: output.map(SemanticType::Scalar),
            kind: TypedExpressionKind::DynamicComparison {
                operator,
                dispatch: Box::new(crate::ir::TypedComparisonDispatch {
                    left_domain,
                    right_domain,
                    resolutions,
                }),
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
            },
            span,
        })
    }

    /// Resolves one candidate complete operand pair into an executable plan.
    ///
    /// Relative addition and subtraction rules are attempted before ordinary
    /// operator resolution, matching the static expression path.
    fn resolve_binary_plan(
        &self,
        operator: CoreBinaryOperator,
        left_type: ValueType,
        right_type: ValueType,
        span: crate::syntax::Span,
    ) -> Result<crate::ir::TypedBinaryPlan, CompileError> {
        let resolution = match operator {
            CoreBinaryOperator::Addition | CoreBinaryOperator::Subtraction => {
                match self
                    .registry
                    .resolve_subtype_relative_rule(operator, left_type, right_type)
                {
                    Ok(relative) => relative,
                    Err(CoreError::SubtypeRelativeOperatorNotDefined { .. }) => self
                        .registry
                        .resolve_binary_operation(operator, left_type, right_type)
                        .map_err(|error| CompileError::core(span, error))?,
                    Err(error) => return Err(CompileError::core(span, error)),
                }
            }
            _ => self
                .registry
                .resolve_binary_operation(operator, left_type, right_type)
                .map_err(|error| CompileError::core(span, error))?,
        };
        let (left_operand_scale, scaled_left_type) =
            self.compile_scale_plan(left_type.base, resolution.left_operand_scale, span)?;
        let right_scale = resolution
            .relative_adjustment
            .unwrap_or(resolution.right_operand_scale);
        let (right_operand_scale, scaled_right_type) =
            self.compile_scale_plan(right_type.base, right_scale, span)?;
        let relative_adjustment_operator = if resolution.relative_adjustment.is_some() {
            Some(
                self.registry
                    .resolve_binary_operator(
                        CoreBinaryOperator::Multiplication,
                        scaled_left_type,
                        scaled_right_type,
                    )
                    .map_err(|error| CompileError::core(span, error))?,
            )
        } else {
            None
        };
        let mut execution_plan = crate::ir::TypedBinaryExecutionPlan {
            left_operand_scale,
            right_operand_scale,
            relative_adjustment_operator,
            result_domain: None,
        };
        execution_plan.result_domain = self.binary_result_domain(&resolution, &execution_plan);
        Ok(crate::ir::TypedBinaryPlan {
            resolution,
            execution_plan,
        })
    }

    /// Compiles unary negation for every complete operand identity in a finite
    /// domain.
    pub(super) fn dynamic_negation(
        &self,
        operand: TypedExpression,
        domain: Arc<CompleteTypeDomain>,
        span: crate::syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let mut plans = Vec::with_capacity(domain.len());
        let mut result_candidates = Vec::new();
        let mut first_error = None;
        let mut output = None;
        for candidate in domain.candidates.iter().copied() {
            let zero = match self
                .registry
                .parse_numeric("0", Some(candidate.base))
                .map(|value| value.with_subtype(candidate.subtype))
                .map_err(|error| CompileError::core(span, error))
            {
                Ok(zero) => zero,
                Err(error) => {
                    first_error.get_or_insert(error);
                    plans.push(None);
                    continue;
                }
            };
            let plan = match self.resolve_binary_plan(
                CoreBinaryOperator::Subtraction,
                candidate,
                candidate,
                span,
            ) {
                Ok(binary) => {
                    output.get_or_insert(binary.resolution.output);
                    result_candidates.push(binary.resolution.output);
                    if let Some(result_domain) = &binary.execution_plan.result_domain {
                        result_candidates.extend(result_domain.candidates.iter().copied());
                    }
                    Some(TypedUnaryNegationPlan { zero, binary })
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                    None
                }
            };
            plans.push(plan);
        }
        let Some(output) = output else {
            return Err(first_error.unwrap_or_else(|| {
                CompileError::new(span, "unary negation is not defined for the operand type")
            }));
        };
        let result_domain = CompleteTypeDomain::from_candidates(self.registry, result_candidates);
        let result_domain = (result_domain.len() > 1).then_some(result_domain);
        Ok(TypedExpression {
            output: Some(SemanticType::Scalar(output)),
            kind: TypedExpressionKind::DynamicNegation {
                dispatch: Box::new(crate::ir::TypedUnaryNegationDispatch {
                    domain,
                    plans,
                    result_domain,
                }),
                operand: Box::new(operand),
            },
            span,
        })
    }

    /// Compiles one conversion whose source complete type is only known at runtime.
    ///
    /// The target is resolved for every candidate source type into a dispatch
    /// table, so runtime execution only selects the precomputed plan.
    pub(crate) fn dynamic_convert(
        &self,
        expression: TypedExpression,
        source_domain: Arc<CompleteTypeDomain>,
        target_base: Option<TypeId>,
        target_subtype: Option<SubtypeId>,
        target_description: String,
        span: crate::syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let mut plans = Vec::with_capacity(source_domain.len());
        let mut reference_output: Option<ValueType> = None;
        let mut result_candidates = Vec::new();
        let mut plain_error: Option<CompileError> = None;
        for candidate_source in source_domain.candidates.iter().copied() {
            let plan = match self.resolve_conversion_plan(
                candidate_source,
                target_base,
                target_subtype,
                span,
            ) {
                Ok(plan) => {
                    reference_output.get_or_insert(plan.conversion.output);
                    result_candidates.push(plan.conversion.output);
                    if target_base.is_none() && !plan.conversion.scale.is_identity() {
                        result_candidates
                            .extend(self.operation_result_candidates(plan.conversion.output, true));
                    }
                    Some(plan)
                }
                Err(error) => {
                    if candidate_source.subtype.is_none() {
                        plain_error = Some(error);
                    }
                    None
                }
            };
            plans.push(plan);
        }
        let Some(output) = reference_output else {
            return Err(plain_error.unwrap_or_else(|| {
                CompileError::new(
                    span,
                    format!("conversion to {target_description} is not defined"),
                )
            }));
        };
        let result_domain = CompleteTypeDomain::from_candidates(self.registry, result_candidates);
        let result_domain = (result_domain.len() > 1).then_some(result_domain);
        Ok(TypedExpression {
            output: Some(SemanticType::Scalar(output)),
            kind: TypedExpressionKind::DynamicConvert {
                dispatch: Box::new(crate::ir::TypedConversionDispatch {
                    target_description,
                    source_domain,
                    plans,
                    result_domain,
                }),
                expression: Box::new(expression),
            },
            span,
        })
    }

    /// Resolves one candidate source type into a conversion plan.
    fn resolve_conversion_plan(
        &self,
        source: ValueType,
        target_base: Option<TypeId>,
        target_subtype: Option<SubtypeId>,
        span: crate::syntax::Span,
    ) -> Result<crate::ir::TypedConversionPlan, CompileError> {
        match (target_base, target_subtype) {
            (Some(base), Some(subtype)) => {
                let resolved = self
                    .registry
                    .resolve_subtype_conversion(source, subtype)
                    .map_err(|error| CompileError::core(span, error))?;
                let output = ValueType::qualified(base, subtype);
                Ok(crate::ir::TypedConversionPlan {
                    conversion: ResolvedSubtypeConversion {
                        output,
                        scale: resolved.scale,
                    },
                    target_base: Some(base),
                    scale_plan: self
                        .compile_scale_plan(source.base, resolved.scale, span)?
                        .0,
                })
            }
            (Some(base), None) => Ok(crate::ir::TypedConversionPlan {
                conversion: ResolvedSubtypeConversion {
                    output: ValueType {
                        base,
                        subtype: source.subtype,
                    },
                    scale: Scale::IDENTITY,
                },
                target_base: Some(base),
                scale_plan: crate::ir::TypedScalePlan::default(),
            }),
            (None, Some(subtype)) => {
                let resolved = self
                    .registry
                    .resolve_subtype_conversion(source, subtype)
                    .map_err(|error| CompileError::core(span, error))?;
                Ok(crate::ir::TypedConversionPlan {
                    scale_plan: self
                        .compile_scale_plan(source.base, resolved.scale, span)?
                        .0,
                    conversion: resolved,
                    target_base: None,
                })
            }
            (None, None) => unreachable!("a conversion always names a type or a unit"),
        }
    }
}
