//! Compilation of expressions and conversions.
//!
//! Every operator, comparison, and conversion is resolved through
//! the registry here, and the per-operation execution plans the runtime replays
//! are built at this stage.

use crate::semantic::{
    BinaryOperator as CoreBinaryOperator, ComparisonOperator as CoreComparisonOperator, CoreError,
    ResolvedSubtypeConversion, Scale, SemanticType, TypeId, ValueType,
};
use crate::syntax::{BinaryOperator, ComparisonOperator, Expression, UnaryOperator};

use crate::CompileError;
use crate::ir::{
    TypedBinaryExecutionPlan, TypedExpression, TypedExpressionKind, TypedScalePlan, TypedScaleStep,
};

use super::Compiler;
use super::helpers::{is_negative_integer_power, is_true_boolean_literal, with_span};

impl Compiler<'_> {
    /// Lowers one source expression into a typed expression with a resolved
    /// semantic type.
    ///
    /// `expected` carries the type requested by the enclosing construct. It
    /// drives numeric literal parsing, so an annotated `12.50` is parsed by the
    /// annotated type instead of passing through a generic float path.
    pub(crate) fn compile_expression(
        &mut self,
        expression: &Expression,
        expected: Option<TypeId>,
    ) -> Result<TypedExpression, CompileError> {
        match expression {
            Expression::Number {
                raw_text,
                suffix,
                span,
            } => {
                let mut value = self
                    .registry
                    .parse_numeric(raw_text, expected)
                    .map_err(|e| CompileError::core(*span, e))?;
                if let Some(suffix) = suffix {
                    let subtype = self.registry.subtype_by_suffix(suffix).ok_or_else(|| {
                        CompileError::new(
                            *span,
                            format!("unknown numeric literal suffix `{suffix}`"),
                        )
                    })?;
                    value = value.with_subtype(Some(subtype));
                }
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(value.value_type())),
                    kind: TypedExpressionKind::Literal(value),
                    span: *span,
                })
            }
            Expression::String { value, span } => {
                let parsed = self
                    .registry
                    .parse_string(value, expected)
                    .map_err(|e| CompileError::core(*span, e))?;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(parsed.value_type())),
                    kind: TypedExpressionKind::Literal(parsed),
                    span: *span,
                })
            }
            Expression::Regex { raw_text, span } => {
                let parsed = self
                    .registry
                    .parse_regex(raw_text, expected)
                    .map_err(|e| CompileError::core(*span, e))?;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(parsed.value_type())),
                    kind: TypedExpressionKind::Literal(parsed),
                    span: *span,
                })
            }
            Expression::Boolean { raw_text, span } => {
                let parsed = self
                    .registry
                    .parse_boolean(raw_text, None)
                    .map_err(|e| CompileError::core(*span, e))?;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(parsed.value_type())),
                    kind: TypedExpressionKind::Literal(parsed),
                    span: *span,
                })
            }
            Expression::Null { span } => {
                let parsed = self
                    .registry
                    .parse_null("null", None)
                    .map_err(|error| CompileError::core(*span, error))?;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(parsed.value_type())),
                    kind: TypedExpressionKind::Literal(parsed),
                    span: *span,
                })
            }
            Expression::Variable { name, span } => {
                let variable = self
                    .resolve_variable(name)
                    .ok_or_else(|| CompileError::new(*span, format!("unknown binding `{name}`")))?;
                let current_type = self.effective_variable_semantic_type(&variable);
                let (output, flow_domain) = match (&variable.contract, &current_type) {
                    (crate::ir::BindingContract::Dynamic, SemanticType::Union(members))
                        if members
                            .iter()
                            .all(|member| matches!(member, SemanticType::Scalar(_))) =>
                    {
                        let candidates = members.iter().filter_map(SemanticType::as_scalar);
                        let domain = crate::ir::CompleteTypeDomain::from_candidates(
                            self.registry,
                            candidates,
                        );
                        (members.first().cloned(), Some(domain))
                    }
                    _ => (Some(current_type), variable.complete_type_domain.clone()),
                };
                Ok(TypedExpression {
                    output,
                    kind: TypedExpressionKind::Variable {
                        name: name.clone(),
                        binding: variable.binding,
                        slot: variable.slot,
                        complete_type_domain: flow_domain,
                    },
                    span: *span,
                })
            }
            Expression::Unary {
                operator,
                operand,
                span,
            } => {
                if *operator == UnaryOperator::Negation
                    && let Some(typed) =
                        self.compile_negative_integer_literal(operand, expected, *span)?
                {
                    return Ok(typed);
                }
                let operand = self.compile_expression(operand, expected)?;
                if operand.is_open_value() {
                    return match operator {
                        UnaryOperator::Negation => Ok(TypedExpression {
                            output: None,
                            kind: TypedExpressionKind::OpenNegation {
                                dispatch: crate::ir::TypedOpenBinaryDispatch::default(),
                                operand: Box::new(operand),
                            },
                            span: *span,
                        }),
                        UnaryOperator::LogicalNot => {
                            let boolean_type = self
                                .registry
                                .default_boolean()
                                .map_err(|error| CompileError::core(*span, error))?;
                            Ok(TypedExpression {
                                output: Some(SemanticType::Scalar(ValueType::plain(boolean_type))),
                                kind: TypedExpressionKind::LogicalNot {
                                    operand: Box::new(operand),
                                },
                                span: *span,
                            })
                        }
                    };
                }
                let operand_type =
                    self.scalar_expression_type(&operand, "unary operand has no value")?;
                match operator {
                    UnaryOperator::Negation => {
                        let plain_zero = self
                            .registry
                            .parse_numeric("0", Some(operand_type.base))
                            .map_err(|error| CompileError::core(*span, error))?;
                        if let Some(domain) = operand.complete_type_domain() {
                            return self.dynamic_negation(operand, domain, *span);
                        }
                        let zero = plain_zero.with_subtype(operand_type.subtype);
                        let resolution = self
                            .registry
                            .resolve_binary_operation(
                                CoreBinaryOperator::Subtraction,
                                zero.value_type(),
                                operand_type,
                            )
                            .map_err(|error| CompileError::core(*span, error))?;
                        Ok(TypedExpression {
                            output: Some(SemanticType::Scalar(resolution.output)),
                            kind: TypedExpressionKind::Binary {
                                resolution,
                                execution_plan: Box::new(TypedBinaryExecutionPlan {
                                    left_operand_scale: TypedScalePlan::default(),
                                    right_operand_scale: TypedScalePlan::default(),
                                    relative_adjustment_operator: None,
                                    result_domain: None,
                                }),
                                left_operand: Box::new(TypedExpression {
                                    output: Some(SemanticType::Scalar(zero.value_type())),
                                    kind: TypedExpressionKind::Literal(zero),
                                    span: *span,
                                }),
                                right_operand: Box::new(operand),
                            },
                            span: *span,
                        })
                    }
                    UnaryOperator::LogicalNot => {
                        self.require_plain_boolean(operand_type, operand.span)?;
                        Ok(TypedExpression {
                            output: Some(SemanticType::Scalar(operand_type)),
                            kind: TypedExpressionKind::LogicalNot {
                                operand: Box::new(operand),
                            },
                            span: *span,
                        })
                    }
                }
            }
            Expression::Binary {
                operator,
                left_operand,
                right_operand,
                span,
            } => {
                let default_string = self.registry.default_string().ok();
                let operand_expected = if expected.is_some_and(|expected| {
                    default_string.is_some_and(|string_type| expected == string_type)
                }) {
                    None
                } else if is_negative_integer_power(operator, right_operand) {
                    Some(
                        self.registry
                            .default_fractional()
                            .map_err(|error| CompileError::core(*span, error))?,
                    )
                } else {
                    expected
                };
                let mut left_operand = self.compile_expression(left_operand, operand_expected)?;
                let mut right_operand = self.compile_expression(right_operand, operand_expected)?;
                let core_binary_operator = match operator {
                    BinaryOperator::Addition => CoreBinaryOperator::Addition,
                    BinaryOperator::Subtraction => CoreBinaryOperator::Subtraction,
                    BinaryOperator::Multiplication => CoreBinaryOperator::Multiplication,
                    BinaryOperator::Division => CoreBinaryOperator::Division,
                    BinaryOperator::Remainder => CoreBinaryOperator::Remainder,
                    BinaryOperator::Power => CoreBinaryOperator::Power,
                };
                if left_operand.is_open_value() || right_operand.is_open_value() {
                    return Ok(TypedExpression {
                        output: None,
                        kind: TypedExpressionKind::OpenBinary {
                            operator: core_binary_operator,
                            dispatch: crate::ir::TypedOpenBinaryDispatch::default(),
                            left_operand: Box::new(left_operand),
                            right_operand: Box::new(right_operand),
                        },
                        span: *span,
                    });
                }
                let mut left_operand_type =
                    self.scalar_expression_type(&left_operand, "left operand has no value")?;
                let mut right_operand_type =
                    self.scalar_expression_type(&right_operand, "right operand has no value")?;

                if core_binary_operator == CoreBinaryOperator::Addition
                    && let Some(string_type) = default_string
                {
                    if left_operand_type.base == string_type
                        && self.accepts_numeric_literals(right_operand_type, right_operand.span)?
                    {
                        right_operand =
                            self.compile_string_conversion(right_operand, string_type)?;
                        right_operand_type = ValueType::plain(string_type);
                    } else if right_operand_type.base == string_type
                        && self.accepts_numeric_literals(left_operand_type, left_operand.span)?
                    {
                        left_operand = self.compile_string_conversion(left_operand, string_type)?;
                        left_operand_type = ValueType::plain(string_type);
                    }
                }

                // Relative addition and subtraction read a qualified right
                // operand as a fraction of the left operand, so consult the
                // relative index before the ordinary operator index. Any other
                // relative failure still reports the relative error.
                let dynamic_operands = left_operand.complete_type_domain().is_some()
                    || right_operand.complete_type_domain().is_some();
                // An operand whose complete type is only known at runtime cannot
                // be resolved to one operator here. The dispatch table instead
                // pre-resolves every candidate subtype so the runtime selects the
                // plan the operand's own subtype calls for.
                if dynamic_operands {
                    let left_domain =
                        self.scalar_complete_type_domain(&left_operand, left_operand_type);
                    let right_domain =
                        self.scalar_complete_type_domain(&right_operand, right_operand_type);
                    return self.dynamic_binary(
                        core_binary_operator,
                        left_operand,
                        left_domain,
                        right_operand,
                        right_domain,
                        *span,
                    );
                }
                let resolution = match core_binary_operator {
                    CoreBinaryOperator::Addition | CoreBinaryOperator::Subtraction => {
                        match self.registry.resolve_subtype_relative_rule(
                            core_binary_operator,
                            left_operand_type,
                            right_operand_type,
                        ) {
                            Ok(relative) => relative,
                            Err(CoreError::SubtypeRelativeOperatorNotDefined { .. }) => self
                                .registry
                                .resolve_binary_operation(
                                    core_binary_operator,
                                    left_operand_type,
                                    right_operand_type,
                                )
                                .map_err(|error| CompileError::core(*span, error))?,
                            Err(error) => return Err(CompileError::core(*span, error)),
                        }
                    }
                    _ => self
                        .registry
                        .resolve_binary_operation(
                            core_binary_operator,
                            left_operand_type,
                            right_operand_type,
                        )
                        .map_err(|error| CompileError::core(*span, error))?,
                };
                if core_binary_operator == CoreBinaryOperator::Multiplication {
                    if is_true_boolean_literal(&right_operand)
                        && left_operand_type == resolution.output
                    {
                        return Ok(with_span(left_operand, *span));
                    }
                    if is_true_boolean_literal(&left_operand)
                        && right_operand_type == resolution.output
                    {
                        return Ok(with_span(right_operand, *span));
                    }
                }
                let (left_operand_scale, scaled_left_operand_type) = self.compile_scale_plan(
                    left_operand_type.base,
                    resolution.left_operand_scale,
                    *span,
                )?;
                let right_scale = resolution
                    .relative_adjustment
                    .unwrap_or(resolution.right_operand_scale);
                let (right_operand_scale, scaled_right_operand_type) =
                    self.compile_scale_plan(right_operand_type.base, right_scale, *span)?;
                let relative_adjustment_operator = if resolution.relative_adjustment.is_some() {
                    Some(
                        self.registry
                            .resolve_binary_operator(
                                CoreBinaryOperator::Multiplication,
                                scaled_left_operand_type,
                                scaled_right_operand_type,
                            )
                            .map_err(|error| CompileError::core(*span, error))?,
                    )
                } else {
                    None
                };
                let mut execution_plan = TypedBinaryExecutionPlan {
                    left_operand_scale,
                    right_operand_scale,
                    relative_adjustment_operator,
                    result_domain: None,
                };
                execution_plan.result_domain =
                    self.binary_result_domain(&resolution, &execution_plan);
                self.fold_literal_scale(&mut left_operand, &mut execution_plan.left_operand_scale)?;
                self.fold_literal_scale(
                    &mut right_operand,
                    &mut execution_plan.right_operand_scale,
                )?;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(resolution.output)),
                    kind: TypedExpressionKind::Binary {
                        resolution,
                        execution_plan: Box::new(execution_plan),
                        left_operand: Box::new(left_operand),
                        right_operand: Box::new(right_operand),
                    },
                    span: *span,
                })
            }
            Expression::Comparison {
                operator,
                left_operand,
                right_operand,
                span,
            } => {
                if matches!(
                    operator,
                    ComparisonOperator::Equal | ComparisonOperator::NotEqual
                ) {
                    // One operand compared with the null literal may hold null.
                    // Which expressions may is decided by the compiled operand
                    // itself, so a nullable binding and a removal from an array
                    // both reach the dedicated null test.
                    let nullable_operand = match (left_operand.as_ref(), right_operand.as_ref()) {
                        (Expression::Null { .. }, other) | (other, Expression::Null { .. })
                            if !matches!(other, Expression::Null { .. }) =>
                        {
                            Some(other)
                        }
                        _ => None,
                    };
                    if let Some(nullable_operand) = nullable_operand {
                        let typed_operand = self.compile_expression(nullable_operand, None)?;
                        // A binding narrowed by an enclosing proof is no longer
                        // nullable, but its declaration still is, and comparing
                        // it with null remains a legitimate test, so both the
                        // compiled operand and the declaration are consulted.
                        let declared_nullable = self.expression_is_nullable(&typed_operand)
                            || match nullable_operand {
                                Expression::Variable { name, .. } => {
                                    self.resolve_variable(name).is_some_and(|variable| {
                                        self.semantic_type_is_nullable(&variable.semantic_type)
                                    })
                                }
                                _ => false,
                            };
                        if declared_nullable || typed_operand.is_open_value() {
                            let boolean_type = self
                                .registry
                                .default_boolean()
                                .map_err(|error| CompileError::core(*span, error))?;
                            return Ok(TypedExpression {
                                output: Some(SemanticType::Scalar(ValueType::plain(boolean_type))),
                                kind: TypedExpressionKind::NullCheck {
                                    operand: Box::new(typed_operand),
                                    equal: matches!(operator, ComparisonOperator::Equal),
                                    boolean_type,
                                },
                                span: *span,
                            });
                        }
                    }
                }
                // The expected type belongs to the boolean result, never to
                // the operands. This preserves literal inference for e.g.
                // `int == decimal` inside a `bool` declaration.
                let left_operand = self.compile_expression(left_operand, None)?;
                let right_operand = self.compile_expression(right_operand, None)?;
                let core_comparison_operator = match operator {
                    ComparisonOperator::Equal => CoreComparisonOperator::Equal,
                    ComparisonOperator::NotEqual => CoreComparisonOperator::NotEqual,
                    ComparisonOperator::Less => CoreComparisonOperator::Less,
                    ComparisonOperator::LessOrEqual => CoreComparisonOperator::LessOrEqual,
                    ComparisonOperator::Greater => CoreComparisonOperator::Greater,
                    ComparisonOperator::GreaterOrEqual => CoreComparisonOperator::GreaterOrEqual,
                };
                if left_operand.is_open_value() || right_operand.is_open_value() {
                    let boolean_type = self
                        .registry
                        .default_boolean()
                        .map_err(|error| CompileError::core(*span, error))?;
                    return Ok(TypedExpression {
                        output: Some(SemanticType::Scalar(ValueType::plain(boolean_type))),
                        kind: TypedExpressionKind::OpenComparison {
                            operator: core_comparison_operator,
                            dispatch: crate::ir::TypedOpenComparisonDispatch::default(),
                            left_operand: Box::new(left_operand),
                            right_operand: Box::new(right_operand),
                        },
                        span: *span,
                    });
                }
                let left_operand_type = self.scalar_expression_type(
                    &left_operand,
                    "left comparison operand has no value",
                )?;
                let right_operand_type = self.scalar_expression_type(
                    &right_operand,
                    "right comparison operand has no value",
                )?;
                let dynamic_operands = left_operand.complete_type_domain().is_some()
                    || right_operand.complete_type_domain().is_some();
                if dynamic_operands {
                    let left_domain =
                        self.scalar_complete_type_domain(&left_operand, left_operand_type);
                    let right_domain =
                        self.scalar_complete_type_domain(&right_operand, right_operand_type);
                    return self.dynamic_comparison(
                        core_comparison_operator,
                        left_operand,
                        left_domain,
                        right_operand,
                        right_domain,
                        *span,
                    );
                }
                let resolution = self
                    .registry
                    .resolve_comparison_operation(
                        core_comparison_operator,
                        left_operand_type,
                        right_operand_type,
                    )
                    .map_err(|error| CompileError::core(*span, error))?;
                let left_operand_scale = self
                    .compile_scale_plan(
                        left_operand_type.base,
                        resolution.left_operand_scale,
                        *span,
                    )?
                    .0;
                let right_operand_scale = self
                    .compile_scale_plan(
                        right_operand_type.base,
                        resolution.right_operand_scale,
                        *span,
                    )?
                    .0;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(resolution.output)),
                    kind: TypedExpressionKind::Comparison {
                        resolution,
                        execution_plan: Box::new(crate::ir::TypedComparisonPlan {
                            resolution,
                            left_operand_scale,
                            right_operand_scale,
                        }),
                        left_operand: Box::new(left_operand),
                        right_operand: Box::new(right_operand),
                    },
                    span: *span,
                })
            }
            Expression::Logical {
                operator,
                left_operand,
                right_operand,
                span,
            } => {
                let left_operand = self.compile_expression(left_operand, None)?;
                let right_operand = self.compile_expression(right_operand, None)?;
                let left_operand_type = self
                    .scalar_expression_type(&left_operand, "left logical operand has no value")?;
                let right_operand_type = self
                    .scalar_expression_type(&right_operand, "right logical operand has no value")?;
                self.require_plain_boolean(left_operand_type, left_operand.span)?;
                self.require_plain_boolean(right_operand_type, right_operand.span)?;
                Ok(TypedExpression {
                    output: Some(SemanticType::Scalar(left_operand_type)),
                    kind: TypedExpressionKind::Logical {
                        operator: *operator,
                        left_operand: Box::new(left_operand),
                        right_operand: Box::new(right_operand),
                    },
                    span: *span,
                })
            }
            Expression::Convert {
                expression,
                target,
                span,
            } => {
                let typed_expression = self.compile_expression(expression, expected)?;
                if let Some(array_type) = typed_expression.array_type() {
                    // A bare `->name` is the argument-less spelling of a method
                    // call, exactly as `->lowercase` calls a string function.
                    // Resolving it through the array method table keeps that
                    // spelling and `->name()` consistent, and reports the
                    // operation's own arity or name diagnostic instead of a
                    // scalar-operand failure.
                    return self.compile_array_method(
                        expression,
                        &typed_expression,
                        array_type,
                        target,
                        &[],
                        *span,
                    );
                }
                let source = self.scalar_expression_type(
                    &typed_expression,
                    "a void expression cannot be converted",
                )?;
                if self.registry.subtype_by_suffix(target).is_some() {
                    return Err(CompileError::new(
                        *span,
                        format!(
                            "bare `->{target}` conversions are no longer supported; use `->to({target})` for units and `->to(type)` or `->to(type, {target})` for representation changes"
                        ),
                    ));
                }
                let function = self
                    .registry
                    .resolve_receiver_function(source.base, target, &[source.base])
                    .map_err(|error| CompileError::core(*span, error))?;
                let output = self
                    .registry
                    .function(function)
                    .map_err(|error| CompileError::core(*span, error))?
                    .output
                    .map(|type_id| SemanticType::Scalar(ValueType::plain(type_id)));
                Ok(TypedExpression {
                    output,
                    kind: TypedExpressionKind::Pipe {
                        function,
                        base: Box::new(typed_expression),
                        arguments: Vec::new(),
                    },
                    span: *span,
                })
            }
            Expression::Pipe {
                expression,
                function,
                arguments,
                span,
            } => {
                if function == "to" {
                    return self.compile_measure_to(expression, arguments, *span);
                }
                let typed_base = self.compile_expression(expression, None)?;
                if let Some(array_type) = typed_base.array_type() {
                    return self.compile_array_method(
                        expression,
                        &typed_base,
                        array_type,
                        function,
                        arguments,
                        *span,
                    );
                }
                let base_type =
                    self.scalar_expression_type(&typed_base, "pipe receiver has no value")?;
                let mut typed_arguments = Vec::with_capacity(arguments.len());
                let mut argument_types = Vec::with_capacity(1 + arguments.len());
                argument_types.push(base_type.base);
                for argument in arguments {
                    let typed = self.compile_expression(argument, None)?;
                    // Nullability is rejected here; whether a container may be
                    // passed is decided once the called function is known.
                    self.require_non_nullable_expression(&typed)?;
                    let base_type = self.expression_base_type(
                        &typed,
                        "void expression cannot be passed as an argument",
                    )?;
                    argument_types.push(base_type);
                    typed_arguments.push(typed);
                }
                let function_id = self
                    .registry
                    .resolve_receiver_function(base_type.base, function, &argument_types)
                    .or_else(|error| match error {
                        crate::semantic::CoreError::UnknownFunction(_) => {
                            self.registry.resolve_function(function, &argument_types)
                        }
                        other => Err(other),
                    })
                    .map_err(|e| CompileError::core(*span, e))?;
                self.require_scalar_arguments(function_id, &typed_arguments, *span)?;
                let output = self
                    .registry
                    .function(function_id)
                    .map_err(|e| CompileError::core(*span, e))?
                    .output
                    .map(|type_id| SemanticType::Scalar(ValueType::plain(type_id)));
                Ok(TypedExpression {
                    output,
                    kind: TypedExpressionKind::Pipe {
                        function: function_id,
                        base: Box::new(typed_base),
                        arguments: typed_arguments,
                    },
                    span: *span,
                })
            }
            Expression::Call {
                namespace,
                function,
                arguments,
                span,
            } => {
                let mut typed_arguments = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let typed = self.compile_expression(argument, None)?;
                    // Nullability is rejected here; whether a container may be
                    // passed is decided once the called function is known.
                    self.require_non_nullable_expression(&typed)?;
                    typed_arguments.push(typed);
                }
                let has_structural_argument = typed_arguments.iter().any(|argument| {
                    argument.is_open_value()
                        || argument.output.as_ref().is_some_and(|semantic_type| {
                            matches!(semantic_type, SemanticType::Union(_))
                                || Self::type_contains_array(semantic_type)
                        })
                });
                if has_structural_argument && typed_arguments.len() != 1 {
                    return Err(CompileError::new(
                        *span,
                        "a structural value can only be passed as the single argument of a generic function",
                    ));
                }
                let function = if has_structural_argument {
                    self.resolve_source_any_single_function(namespace.as_ref(), function, *span)?
                } else {
                    let argument_types = typed_arguments
                        .iter()
                        .map(|typed| {
                            self.expression_base_type(
                                typed,
                                "void expression cannot be passed as an argument",
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    self.resolve_source_function(
                        namespace.as_ref(),
                        function,
                        &argument_types,
                        *span,
                    )?
                };
                self.require_scalar_arguments(function, &typed_arguments, *span)?;
                let output = self
                    .registry
                    .function(function)
                    .map_err(|e| CompileError::core(*span, e))?
                    .output
                    .map(|type_id| SemanticType::Scalar(ValueType::plain(type_id)));
                Ok(TypedExpression {
                    output,
                    kind: TypedExpressionKind::Call {
                        function,
                        arguments: typed_arguments,
                    },
                    span: *span,
                })
            }
            Expression::ArrayLiteral { .. } => self.compile_array_expression(expression, None),
            Expression::ElementAccess {
                expression: array,
                index,
                span,
            } => {
                let typed_array = self.compile_expression(array, None)?;
                let array_types = self.array_types_for_access(&typed_array).ok_or_else(|| {
                    CompileError::new(array.span(), "only an array can be indexed with `[]`")
                })?;
                let array_type = array_types.first().cloned();
                let (typed_index, constant_index, index_extractor, index_dispatch) =
                    self.compile_array_index(index)?;
                let (output, type_domain) = if array_types.len() == 1 {
                    let array_type = array_type.expect("one array contract was collected");
                    let (output, type_domain) = match self
                        .array_element_type(&typed_array, constant_index)
                    {
                        // A recorded literal element keeps its own complete type,
                        // including a widened adaptive representation.
                        Some(Some(element_type)) => self
                            .finite_array_element_output([element_type])
                            .expect("one recorded element type is a finite domain"),
                        // Exact constrained contracts fix both axes. Adaptive
                        // constrained contracts fix only the subtype; a value may
                        // still carry a promoted signed base.
                        Some(None) | None => match array_type.static_semantic_type() {
                            Some(element_type) => {
                                let dynamic = matches!(
                                    element_type,
                                    SemanticType::Scalar(element)
                                        if element.subtype.is_none()
                                            || array_type.static_representation()
                                                == Some(crate::semantic::ScalarRepresentation::AdaptiveSignedInteger)
                                );
                                (
                                    element_type.clone(),
                                    dynamic.then(|| {
                                        self.array_element_complete_type_domain(array_type.clone())
                                    }),
                                )
                            }
                            None => {
                                let Some(known) = self.array_known_element_types(&typed_array)
                                else {
                                    return Ok(TypedExpression {
                                        output: None,
                                        kind: TypedExpressionKind::ElementAccess {
                                            array: Box::new(typed_array),
                                            index: Box::new(typed_index),
                                            constant_index,
                                            index_extractor,
                                            type_domain: None,
                                            index_dispatch: index_dispatch.map(Box::new),
                                        },
                                        span: *span,
                                    });
                                };
                                match self.finite_array_element_output(known) {
                                    Some(known) => known,
                                    None => {
                                        return Ok(TypedExpression {
                                            output: None,
                                            kind: TypedExpressionKind::ElementAccess {
                                                array: Box::new(typed_array),
                                                index: Box::new(typed_index),
                                                constant_index,
                                                index_extractor,
                                                type_domain: None,
                                                index_dispatch: index_dispatch.map(Box::new),
                                            },
                                            span: *span,
                                        });
                                    }
                                }
                            }
                        },
                    };
                    (output, type_domain)
                } else {
                    (
                        SemanticType::union(
                            array_types.iter().filter_map(|array_type| {
                                array_type.static_semantic_type().cloned()
                            }),
                        ),
                        None,
                    )
                };
                Ok(TypedExpression {
                    output: Some(output),
                    kind: TypedExpressionKind::ElementAccess {
                        array: Box::new(typed_array),
                        index: Box::new(typed_index),
                        constant_index,
                        index_extractor,
                        type_domain,
                        index_dispatch: index_dispatch.map(Box::new),
                    },
                    span: *span,
                })
            }
            _ => Err(CompileError::new(
                expression.span(),
                "expression requires an optional language patch",
            )),
        }
    }

    /// Returns whether a value's registered base type accepts numeric literals.
    fn accepts_numeric_literals(
        &self,
        value_type: ValueType,
        span: crate::syntax::Span,
    ) -> Result<bool, CompileError> {
        self.registry
            .type_descriptor(value_type.base)
            .map(|descriptor| descriptor.parse_numeric_literal.is_some())
            .map_err(|error| CompileError::core(span, error))
    }

    /// Compiles a parser-shaped negative integer literal in its signed context.
    ///
    /// The parser represents `-128` as negation applied to the positive token
    /// `128`. Parsing the signed spelling first preserves valid minimum values
    /// such as `int8`'s `-128`; non-integer or non-literal negation keeps the
    /// ordinary unary-expression path.
    fn compile_negative_integer_literal(
        &self,
        operand: &Expression,
        expected: Option<TypeId>,
        span: crate::syntax::Span,
    ) -> Result<Option<TypedExpression>, CompileError> {
        let Expression::Number {
            raw_text, suffix, ..
        } = operand
        else {
            return Ok(None);
        };
        if raw_text.contains(['.', 'e', 'E']) {
            return Ok(None);
        }
        let target = match expected {
            Some(target) => target,
            None => {
                let Some(target) = self.registry.default_integer().ok() else {
                    return Ok(None);
                };
                target
            }
        };
        if !self
            .registry
            .is_integer_type(target)
            .map_err(|error| CompileError::core(span, error))?
        {
            return Ok(None);
        }
        let mut value = self
            .registry
            .parse_numeric(&format!("-{raw_text}"), Some(target))
            .map_err(|error| CompileError::core(span, error))?;
        if let Some(suffix) = suffix {
            let subtype = self.registry.subtype_by_suffix(suffix).ok_or_else(|| {
                CompileError::new(span, format!("unknown numeric literal suffix `{suffix}`"))
            })?;
            value = value.with_subtype(Some(subtype));
        }
        Ok(Some(TypedExpression {
            output: Some(SemanticType::Scalar(value.value_type())),
            kind: TypedExpressionKind::Literal(value),
            span,
        }))
    }

    /// Compiles `receiver->to(...)` measure conversions.
    ///
    /// Each argument must be a bare type name or unit suffix, in any order.
    /// `->to(decimal)` changes only the numeric representation, `->to(MB)`
    /// changes only the unit with the usual fractional promotion, and
    /// `->to(decimal, MB)` changes both by scaling first and then casting the
    /// magnitude to the requested representation.
    fn compile_measure_to(
        &mut self,
        receiver: &Expression,
        arguments: &[Expression],
        span: crate::syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        if arguments.is_empty() || arguments.len() > 2 {
            return Err(CompileError::new(
                span,
                "expected `->to(type)`, `->to(unit)`, or `->to(type, unit)` with one or two targets",
            ));
        }
        let typed_receiver = self.compile_expression(receiver, None)?;
        let source =
            self.scalar_expression_type(&typed_receiver, "a void expression cannot be converted")?;
        let mut target_base: Option<TypeId> = None;
        let mut target_subtype: Option<crate::semantic::SubtypeId> = None;
        let mut target_names: Vec<String> = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let Expression::Variable { name, .. } = argument else {
                return Err(CompileError::new(
                    argument.span(),
                    "conversion targets must be bare type or unit names, e.g. `->to(decimal)`, `->to(MB)`, or `->to(decimal, MB)`",
                ));
            };
            target_names.push(name.clone());
            let is_type = self.registry.type_by_name(name).is_some();
            let is_subtype = self.registry.subtype_by_suffix(name).is_some();
            match (is_type, is_subtype) {
                (false, false) => {
                    return Err(CompileError::new(
                        argument.span(),
                        format!(
                            "unknown conversion target `{name}`; expected a type name or unit suffix"
                        ),
                    ));
                }
                (true, true) => {
                    return Err(CompileError::new(
                        argument.span(),
                        format!(
                            "ambiguous conversion target `{name}`; it names both a type and a unit"
                        ),
                    ));
                }
                (true, false) => {
                    if target_base.is_some() {
                        return Err(CompileError::new(
                            argument.span(),
                            "expected at most one type target in `->to(...)`",
                        ));
                    }
                    let id = self
                        .registry
                        .type_by_name(name)
                        .expect("type checked above");
                    let supports_numeric = self
                        .registry
                        .type_descriptor(id)
                        .map(|descriptor| descriptor.parse_numeric_literal.is_some())
                        .unwrap_or(false);
                    if !supports_numeric {
                        return Err(CompileError::new(
                            argument.span(),
                            format!(
                                "conversion target `{name}` must be a numeric type for `->to(...)`"
                            ),
                        ));
                    }
                    target_base = Some(id);
                }
                (false, true) => {
                    if target_subtype.is_some() {
                        return Err(CompileError::new(
                            argument.span(),
                            "expected at most one unit target in `->to(...)`",
                        ));
                    }
                    target_subtype = Some(
                        self.registry
                            .subtype_by_suffix(name)
                            .expect("subtype checked above"),
                    );
                }
            }
        }
        // An extracted element keeps the subtype it was stored with, so the
        // target must be resolved for every candidate source subtype instead of
        // the declared base alone.
        if let Some(source_domain) = typed_receiver.complete_type_domain() {
            let target_description = format!("`->to({})`", target_names.join(", "));
            return self.dynamic_convert(
                typed_receiver,
                source_domain,
                target_base,
                target_subtype,
                target_description,
                span,
            );
        }
        let (conversion, output) = match (target_base, target_subtype) {
            (Some(base), Some(subtype)) => {
                let resolved = self
                    .registry
                    .resolve_subtype_conversion(source, subtype)
                    .map_err(|error| CompileError::core(span, error))?;
                let output = ValueType::qualified(base, subtype);
                let conversion = ResolvedSubtypeConversion {
                    output,
                    scale: resolved.scale,
                };
                (conversion, output)
            }
            (Some(base), None) => {
                let output = ValueType {
                    base,
                    subtype: source.subtype,
                };
                let conversion = ResolvedSubtypeConversion {
                    output,
                    scale: Scale::IDENTITY,
                };
                (conversion, output)
            }
            (None, Some(subtype)) => {
                let resolved = self
                    .registry
                    .resolve_subtype_conversion(source, subtype)
                    .map_err(|error| CompileError::core(span, error))?;
                (resolved, resolved.output)
            }
            (None, None) => unreachable!("argument count validated above"),
        };
        let (scale_plan, _) = self.compile_scale_plan(source.base, conversion.scale, span)?;
        Ok(TypedExpression {
            output: Some(SemanticType::Scalar(output)),
            kind: TypedExpressionKind::Convert {
                conversion,
                target_base,
                scale_plan,
                expression: Box::new(typed_receiver),
            },
            span,
        })
    }

    /// Resolves and parses every operation needed to apply one subtype scale.
    pub(crate) fn compile_scale_plan(
        &self,
        base_type: TypeId,
        scale: Scale,
        span: crate::syntax::Span,
    ) -> Result<(TypedScalePlan, TypeId), CompileError> {
        let mut scaled_type = base_type;
        let numerator = if scale.numerator == 1 {
            None
        } else {
            let factor = self
                .registry
                .parse_numeric(&scale.numerator.to_string(), Some(scaled_type))
                .map_err(|error| CompileError::core(span, error))?;
            let operator = self
                .registry
                .resolve_binary_operator(
                    CoreBinaryOperator::Multiplication,
                    scaled_type,
                    factor.type_id(),
                )
                .map_err(|error| CompileError::core(span, error))?;
            scaled_type = self
                .registry
                .operator(operator)
                .map_err(|error| CompileError::core(span, error))?
                .result_type;
            Some(TypedScaleStep { operator, factor })
        };
        let denominator = if scale.denominator == 1 {
            None
        } else {
            let divisor_type = if self.registry.default_integer().ok() == Some(scaled_type) {
                self.registry
                    .default_fractional()
                    .map_err(|error| CompileError::core(span, error))?
            } else {
                scaled_type
            };
            let factor = self
                .registry
                .parse_numeric(&scale.denominator.to_string(), Some(divisor_type))
                .map_err(|error| CompileError::core(span, error))?;
            let operator = self
                .registry
                .resolve_binary_operator(
                    CoreBinaryOperator::Division,
                    scaled_type,
                    factor.type_id(),
                )
                .map_err(|error| CompileError::core(span, error))?;
            scaled_type = self
                .registry
                .operator(operator)
                .map_err(|error| CompileError::core(span, error))?
                .result_type;
            Some(TypedScaleStep { operator, factor })
        };
        Ok((
            TypedScalePlan {
                numerator,
                denominator,
            },
            scaled_type,
        ))
    }

    /// Precomputes a literal magnitude scale when every step is context-free.
    fn fold_literal_scale(
        &self,
        expression: &mut TypedExpression,
        scale_plan: &mut TypedScalePlan,
    ) -> Result<(), CompileError> {
        if scale_plan.is_identity() {
            return Ok(());
        }
        let TypedExpressionKind::Literal(literal) = &expression.kind else {
            return Ok(());
        };
        for step in [
            scale_plan.numerator.as_ref(),
            scale_plan.denominator.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if self
                .registry
                .operator(step.operator)
                .map_err(|error| CompileError::core(expression.span, error))?
                .context_execute
                .is_some()
            {
                return Ok(());
            }
        }

        let mut scaled = literal.clone().with_subtype(None);
        for step in [
            scale_plan.numerator.as_ref(),
            scale_plan.denominator.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let descriptor = self
                .registry
                .operator(step.operator)
                .map_err(|error| CompileError::core(expression.span, error))?;
            scaled = (descriptor.execute)(&scaled, &step.factor)
                .map_err(|error| CompileError::core(expression.span, error))?;
        }
        expression.kind = TypedExpressionKind::Literal(scaled);
        *scale_plan = TypedScalePlan::default();
        Ok(())
    }

    /// Wraps one numeric expression in the registered universal string function.
    fn compile_string_conversion(
        &self,
        expression: TypedExpression,
        string_type: TypeId,
    ) -> Result<TypedExpression, CompileError> {
        let argument_type = self.scalar_expression_type(
            &expression,
            "a void expression cannot be converted to string",
        )?;
        let function = self
            .registry
            .resolve_function("string", &[argument_type.base])
            .map_err(|error| CompileError::core(expression.span, error))?;
        let function_output = self
            .registry
            .function(function)
            .map_err(|error| CompileError::core(expression.span, error))?
            .output;
        if function_output != Some(string_type) {
            return Err(CompileError::new(
                expression.span,
                "the registered `string` conversion must return the default string type",
            ));
        }
        let span = expression.span;
        Ok(TypedExpression {
            kind: TypedExpressionKind::Call {
                function,
                arguments: vec![expression],
            },
            output: Some(SemanticType::Scalar(ValueType::plain(string_type))),
            span,
        })
    }
}

#[cfg(test)]
#[path = "expressions.tests.rs"]
mod tests;
