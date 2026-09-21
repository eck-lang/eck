use crate::ir::{
    TypedBinaryExecutionPlan, TypedBinaryPlan, TypedComparisonPlan, TypedExpression,
    TypedExpressionKind, TypedOpenBinaryDispatch, TypedOpenComparisonDispatch, TypedScalePlan,
    TypedScaleStep,
};
use crate::semantic::{
    BinaryOperator, BinaryOperatorDescriptor, ComparisonOperator, CoreError, ExecutionContext,
    ResolvedBinaryOperator, Scale, Value, ValueType,
};
use crate::syntax::LogicalOperator;

use super::Runtime;
use crate::RuntimeError;

impl<'registry> Runtime<'registry> {
    pub(crate) fn eval(
        &mut self,
        expression: &TypedExpression,
    ) -> Result<Option<Value>, RuntimeError> {
        match &expression.kind {
            TypedExpressionKind::Literal(value) => Ok(Some(value.clone())),
            TypedExpressionKind::Variable { name, slot, .. } => self.local_values[slot.0]
                .clone()
                .map(Some)
                .ok_or_else(|| RuntimeError::Message(format!("unknown runtime variable `{name}`"))),
            TypedExpressionKind::ArrayBoundary {
                array_type,
                expression,
            } => {
                let value = self.eval(expression)?.ok_or_else(|| {
                    RuntimeError::Message("array boundary expression returned no value".into())
                })?;
                self.retype_array_value(value, array_type.clone()).map(Some)
            }
            TypedExpressionKind::Binary {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
                ..
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right operand returned no value".into())
                })?;
                let value = self.execute_compiled_binary(
                    resolution,
                    execution_plan,
                    left_operand,
                    right_operand,
                )?;
                Ok(Some(value))
            }
            TypedExpressionKind::DynamicBinary {
                operator,
                dispatch,
                left_operand,
                right_operand,
                ..
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right operand returned no value".into())
                })?;
                let plan =
                    self.select_binary_plan(*operator, dispatch, &left_operand, &right_operand)?;
                let value = self.execute_compiled_binary(
                    &plan.resolution,
                    &plan.execution_plan,
                    left_operand,
                    right_operand,
                )?;
                Ok(Some(value))
            }
            TypedExpressionKind::OpenBinary {
                operator,
                dispatch,
                left_operand,
                right_operand,
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right operand returned no value".into())
                })?;
                let plan =
                    self.open_binary_plan(*operator, dispatch, &left_operand, &right_operand)?;
                self.execute_compiled_binary(
                    &plan.resolution,
                    &plan.execution_plan,
                    left_operand,
                    right_operand,
                )
                .map(Some)
            }
            TypedExpressionKind::DynamicNegation { dispatch, operand } => {
                let operand = self.eval(operand)?.ok_or_else(|| {
                    RuntimeError::Message("unary negation operand returned no value".into())
                })?;
                let slot = dispatch
                    .domain
                    .candidate_index(self.registry, operand.value_type())
                    .ok_or_else(|| {
                        RuntimeError::Message(format!(
                            "unary negation is not defined for `{}`",
                            self.registry.value_type_name(operand.value_type())
                        ))
                    })?;
                let plan = dispatch
                    .plans
                    .get(slot)
                    .and_then(|plan| plan.as_ref())
                    .ok_or_else(|| {
                        RuntimeError::Message(format!(
                            "unary negation is not defined for `{}`",
                            self.registry.value_type_name(operand.value_type())
                        ))
                    })?;
                let value = self.execute_compiled_binary(
                    &plan.binary.resolution,
                    &plan.binary.execution_plan,
                    plan.zero.clone(),
                    operand,
                )?;
                Ok(Some(value))
            }
            TypedExpressionKind::OpenNegation { dispatch, operand } => {
                let operand = self.eval(operand)?.ok_or_else(|| {
                    RuntimeError::Message("unary negation operand returned no value".into())
                })?;
                let operand_type = operand.scalar_type().ok_or_else(|| {
                    RuntimeError::Message("unary negation is not defined for an array".into())
                })?;
                let zero = self
                    .registry
                    .parse_numeric("0", Some(operand_type.base))?
                    .with_subtype(operand_type.subtype);
                let plan =
                    self.open_binary_plan(BinaryOperator::Subtraction, dispatch, &zero, &operand)?;
                self.execute_compiled_binary(&plan.resolution, &plan.execution_plan, zero, operand)
                    .map(Some)
            }
            TypedExpressionKind::Comparison {
                execution_plan,
                left_operand,
                right_operand,
                ..
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left comparison operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right comparison operand returned no value".into())
                })?;
                let value = self.execute_compiled_comparison(
                    execution_plan,
                    &left_operand,
                    &right_operand,
                )?;
                Ok(Some(value))
            }
            TypedExpressionKind::DynamicComparison {
                operator,
                dispatch,
                left_operand,
                right_operand,
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left comparison operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right comparison operand returned no value".into())
                })?;
                let resolution = self.select_comparison_resolution(
                    *operator,
                    dispatch,
                    &left_operand,
                    &right_operand,
                )?;
                let value =
                    self.execute_compiled_comparison(resolution, &left_operand, &right_operand)?;
                Ok(Some(value))
            }
            TypedExpressionKind::OpenComparison {
                operator,
                dispatch,
                left_operand,
                right_operand,
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left comparison operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right comparison operand returned no value".into())
                })?;
                let plan =
                    self.open_comparison_plan(*operator, dispatch, &left_operand, &right_operand)?;
                self.execute_compiled_comparison(&plan, &left_operand, &right_operand)
                    .map(Some)
            }
            TypedExpressionKind::NullCheck {
                operand,
                equal,
                boolean_type,
            } => {
                let value = self.eval(operand)?.ok_or_else(|| {
                    RuntimeError::Message("null-check operand returned no value".into())
                })?;
                let is_null =
                    value.scalar_type() == Some(ValueType::plain(self.registry.default_null()?));
                Ok(Some(Value::new(*boolean_type, is_null == *equal)))
            }
            TypedExpressionKind::Logical {
                operator,
                left_operand,
                right_operand,
            } => {
                let left_value = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left logical operand returned no value".into())
                })?;
                let left_boolean = self.registry.evaluate_boolean(&left_value)?;
                match operator {
                    LogicalOperator::And if !left_boolean => Ok(Some(left_value)),
                    LogicalOperator::Or if left_boolean => Ok(Some(left_value)),
                    LogicalOperator::And | LogicalOperator::Or => self.eval(right_operand),
                }
            }
            TypedExpressionKind::LogicalNot { operand } => {
                let value = self.eval(operand)?.ok_or_else(|| {
                    RuntimeError::Message("logical negation operand returned no value".into())
                })?;
                let boolean = self.registry.evaluate_boolean(&value)?;
                Ok(Some(Value::new(value.type_id(), !boolean)))
            }
            TypedExpressionKind::Convert {
                conversion,
                target_base,
                scale_plan,
                expression,
            } => {
                let value = self.eval(expression)?.ok_or_else(|| {
                    RuntimeError::Message("converted expression returned no value".into())
                })?;
                let exact_integer_target =
                    self.integer_conversion_target(target_base.unwrap_or(conversion.output.base))?;
                let scaled = self
                    .execute_scale_plan_with_target(&value, scale_plan, exact_integer_target)?
                    .with_subtype(None);
                let cast = match target_base {
                    Some(target) => self.registry.convert_base(&scaled, *target)?,
                    None => scaled,
                };
                let value = cast.with_subtype(conversion.output.subtype);
                let value = self
                    .registry
                    .transform_owned_configured_result(value, &self.configuration)?;
                Ok(Some(value))
            }
            TypedExpressionKind::DynamicConvert {
                dispatch,
                expression,
            } => {
                let value = self.eval(expression)?.ok_or_else(|| {
                    RuntimeError::Message("converted expression returned no value".into())
                })?;
                // The stored subtype selects the conversion the compiler
                // pre-resolved for that candidate, so no subtype conversion is
                // resolved during execution.
                let slot = dispatch
                    .source_domain
                    .candidate_index(self.registry, value.value_type())
                    .ok_or_else(|| {
                        RuntimeError::Message(format!(
                            "conversion from `{}` to {} is not defined",
                            self.registry.value_type_name(value.value_type()),
                            dispatch.target_description
                        ))
                    })?;
                let plan = dispatch
                    .plans
                    .get(slot)
                    .and_then(|plan| plan.as_ref())
                    .ok_or_else(|| {
                        RuntimeError::Message(format!(
                            "conversion from `{}` to {} is not defined",
                            self.registry.value_type_name(value.value_type()),
                            dispatch.target_description
                        ))
                    })?;
                let exact_integer_target = self.integer_conversion_target(
                    plan.target_base.unwrap_or(plan.conversion.output.base),
                )?;
                let scaled = self
                    .execute_scale_plan_with_target(&value, &plan.scale_plan, exact_integer_target)?
                    .with_subtype(None);
                let cast = match plan.target_base {
                    Some(target) => self.registry.convert_base(&scaled, target)?,
                    None => scaled,
                };
                let value = cast.with_subtype(plan.conversion.output.subtype);
                let value = self
                    .registry
                    .transform_owned_configured_result(value, &self.configuration)?;
                Ok(Some(value))
            }
            TypedExpressionKind::Call {
                function,
                arguments,
            } => {
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    values.push(self.eval(argument)?.ok_or_else(|| {
                        RuntimeError::Message("argument returned no value".into())
                    })?);
                }
                let function = self.registry.function(*function)?;
                let context = ExecutionContext::new(self.registry, &self.configuration);
                let result = (function.execute)(&context, &values)?;
                debug_assert!(
                    result
                        .as_ref()
                        .is_none_or(|value| self.registry.validate_value(value).is_ok()),
                    "function `{}` produced a value with an unregistered type",
                    function.name
                );
                Ok(result)
            }
            TypedExpressionKind::Pipe {
                function,
                base,
                arguments,
            } => {
                let base_value = self.eval(base)?.ok_or_else(|| {
                    RuntimeError::Message("pipe receiver returned no value".into())
                })?;
                let mut values = Vec::with_capacity(1 + arguments.len());
                values.push(base_value);
                for argument in arguments {
                    values.push(self.eval(argument)?.ok_or_else(|| {
                        RuntimeError::Message("pipe argument returned no value".into())
                    })?);
                }
                let function = self.registry.function(*function)?;
                let context = ExecutionContext::new(self.registry, &self.configuration);
                let result = (function.execute)(&context, &values)?;
                debug_assert!(
                    result
                        .as_ref()
                        .is_none_or(|value| self.registry.validate_value(value).is_ok()),
                    "function `{}` produced a value with an unregistered type",
                    function.name
                );
                Ok(result)
            }
            TypedExpressionKind::ArrayLiteral { elements } => {
                self.build_array_value(expression, elements)
            }
            TypedExpressionKind::ElementStore {
                element,
                expression,
            } => self.store_element_value(*element, expression),
            TypedExpressionKind::ArrayMethod {
                method,
                slot,
                arguments,
                empty_result,
                ..
            } => self.execute_array_method(*method, *slot, arguments, empty_result),
            TypedExpressionKind::ElementAccess {
                array,
                index,
                constant_index,
                index_extractor,
                index_dispatch,
                ..
            } => self.read_array_element(
                array,
                index,
                *constant_index,
                *index_extractor,
                index_dispatch.as_deref(),
            ),
        }
    }

    /// Returns the target when a conversion must preserve an integer magnitude.
    fn integer_conversion_target(
        &self,
        target: crate::semantic::TypeId,
    ) -> Result<Option<crate::semantic::TypeId>, RuntimeError> {
        Ok(self.registry.is_integer_type(target)?.then_some(target))
    }

    /// Rejects an inexact integer subtype conversion before division truncates.
    fn require_exact_integer_division(
        &self,
        dividend: &Value,
        divisor: &Value,
        target: crate::semantic::TypeId,
    ) -> Result<(), RuntimeError> {
        let operator = self.registry.resolve_binary_operator(
            BinaryOperator::Remainder,
            dividend.type_id(),
            divisor.type_id(),
        )?;
        let descriptor = self.registry.operator(operator)?;
        let remainder = self.execute_binary_operator(descriptor, dividend, divisor)?;
        if self.registry.format_value(&remainder)? != "0" {
            return Err(RuntimeError::Message(format!(
                "cannot convert `{}` to `{}`",
                self.registry.value_type_name(dividend.value_type()),
                self.registry.type_name(target)
            )));
        }
        Ok(())
    }

    /// Applies a compiler-prepared magnitude scale without parsing or signature lookup.
    fn execute_scale_plan(
        &self,
        value: &Value,
        scale_plan: &TypedScalePlan,
    ) -> Result<Value, RuntimeError> {
        self.execute_scale_plan_with_target(value, scale_plan, None)
    }

    /// Applies a compiler-prepared scale and optionally checks exact integer division.
    fn execute_scale_plan_with_target(
        &self,
        value: &Value,
        scale_plan: &TypedScalePlan,
        exact_integer_target: Option<crate::semantic::TypeId>,
    ) -> Result<Value, RuntimeError> {
        let mut scaled = value.clone().with_subtype(None);
        if let Some(step) = scale_plan.numerator.as_ref() {
            let descriptor = self.registry.operator(step.operator)?;
            let descriptor = self.redispatch_for_dynamic_types(descriptor, &scaled, &step.factor);
            scaled = self.execute_binary_operator(descriptor, &scaled, &step.factor)?;
        }
        if let Some(step) = scale_plan.denominator.as_ref() {
            if let Some(target) = exact_integer_target
                && self.registry.is_integer_type(scaled.type_id())?
                && self.registry.is_integer_type(step.factor.type_id())?
            {
                self.require_exact_integer_division(&scaled, &step.factor, target)?;
            }
            let descriptor = self.registry.operator(step.operator)?;
            let descriptor = self.redispatch_for_dynamic_types(descriptor, &scaled, &step.factor);
            scaled = self.execute_binary_operator(descriptor, &scaled, &step.factor)?;
        }
        Ok(scaled)
    }

    /// Returns whether a prepared scale includes a registry-aware executor.
    pub(super) fn scale_plan_uses_context(
        &self,
        scale_plan: &TypedScalePlan,
    ) -> Result<bool, RuntimeError> {
        for step in [
            scale_plan.numerator.as_ref(),
            scale_plan.denominator.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if self
                .registry
                .operator(step.operator)?
                .context_execute
                .is_some()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Executes one prepared binary plan from owned stack operands.
    pub(super) fn execute_binary_plan_owned(
        &self,
        resolution: &ResolvedBinaryOperator,
        execution_plan: &TypedBinaryExecutionPlan,
        outer_descriptor: &BinaryOperatorDescriptor,
        adjustment_descriptor: Option<&BinaryOperatorDescriptor>,
        left_operand: Value,
        right_operand: Value,
    ) -> Result<Value, RuntimeError> {
        let mut left_magnitude =
            self.execute_scale_plan_owned(left_operand, &execution_plan.left_operand_scale)?;
        let right_magnitude =
            self.execute_scale_plan_owned(right_operand, &execution_plan.right_operand_scale)?;
        let outer_right = if execution_plan.relative_adjustment_operator.is_some() {
            let descriptor = adjustment_descriptor
                .expect("a relative direct instruction has an adjustment descriptor");
            self.execute_binary_operator(descriptor, &left_magnitude, &right_magnitude)?
        } else {
            right_magnitude
        };
        let descriptor = outer_descriptor;
        if left_magnitude.is_uniquely_owned()
            && let Some(execute) = descriptor.in_place_execute
        {
            execute(&mut left_magnitude, &outer_right)?;
            return Ok(left_magnitude.with_subtype(resolution.output.subtype));
        }
        Ok(self
            .execute_binary_operator(descriptor, &left_magnitude, &outer_right)?
            .with_subtype(resolution.output.subtype))
    }

    /// Applies a prepared scale while reusing an exclusively owned payload when possible.
    fn execute_scale_plan_owned(
        &self,
        value: Value,
        scale_plan: &TypedScalePlan,
    ) -> Result<Value, RuntimeError> {
        let mut scaled = value.with_subtype(None);
        for step in [
            scale_plan.numerator.as_ref(),
            scale_plan.denominator.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let descriptor = self.registry.operator(step.operator)?;
            if scaled.is_uniquely_owned()
                && let Some(execute) = descriptor.in_place_execute
            {
                execute(&mut scaled, &step.factor)?;
            } else {
                scaled = self.execute_binary_operator(descriptor, &scaled, &step.factor)?;
            }
        }
        Ok(scaled)
    }

    /// Executes one resolved binary operator, preferring its registry-aware override.
    ///
    /// Descriptors without a context override run their plain callback exactly
    /// as before, so this helper preserves behavior for every operator that
    /// does not need execution services.
    pub(super) fn execute_binary_operator(
        &self,
        descriptor: &BinaryOperatorDescriptor,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<Value, CoreError> {
        match descriptor.context_execute {
            Some(execute_with_context) => {
                let context = ExecutionContext::new(self.registry, &self.configuration);
                let value = execute_with_context(&context, left_operand, right_operand)?;
                debug_assert!(
                    self.registry.validate_value(&value).is_ok(),
                    "operator `{}` produced a value with an unregistered type",
                    descriptor.operator
                );
                Ok(value)
            }
            None => {
                let value = (descriptor.execute)(left_operand, right_operand)?;
                debug_assert!(
                    self.registry.validate_value(&value).is_ok(),
                    "operator `{}` produced a value with an unregistered type",
                    descriptor.operator
                );
                Ok(value)
            }
        }
    }

    /// Re-resolves a statically dispatched operator when runtime types diverge.
    ///
    /// Dynamic result promotion (for example `int128` overflowing to `bigint`)
    /// can produce operand values whose base types differ from the statically
    /// resolved signature. Re-resolution reuses the already registered operator
    /// for the dynamic pair, which keeps chained expressions working without
    /// changing compile-time types. The original descriptor is returned when
    /// the types still match or no operator exists for the dynamic pair, so
    /// unrelated failures keep their previous error.
    pub(super) fn redispatch_for_dynamic_types<'runtime>(
        &'runtime self,
        descriptor: &'runtime BinaryOperatorDescriptor,
        left_operand: &Value,
        right_operand: &Value,
    ) -> &'runtime BinaryOperatorDescriptor {
        if left_operand.type_id() == descriptor.left_operand_type
            && right_operand.type_id() == descriptor.right_operand_type
        {
            return descriptor;
        }
        self.registry
            .resolve_binary_operator(
                descriptor.operator,
                left_operand.type_id(),
                right_operand.type_id(),
            )
            .and_then(|operator| self.registry.operator(operator))
            .unwrap_or(descriptor)
    }

    /// Executes one compiler-resolved binary plan and applies configuration.
    ///
    /// The plan already carries the resolved operator, the operand scales, and
    /// the optional relative-adjustment operator, so this performs no registry
    /// resolution beyond the descriptor lookups the plan recorded.
    pub(super) fn execute_compiled_binary(
        &self,
        resolution: &ResolvedBinaryOperator,
        execution_plan: &TypedBinaryExecutionPlan,
        left_operand: Value,
        right_operand: Value,
    ) -> Result<Value, RuntimeError> {
        let resolved_operator = self.registry.operator(resolution.operator)?;
        let direct_execution = resolution.relative_adjustment.is_none()
            && resolution.left_operand_scale.is_identity()
            && resolution.right_operand_scale.is_identity()
            && left_operand.subtype_id().is_none()
            && right_operand.subtype_id().is_none()
            && resolution.output.subtype.is_none()
            && left_operand.type_id() == resolved_operator.left_operand_type
            && right_operand.type_id() == resolved_operator.right_operand_type;
        let value = if direct_execution {
            self.execute_binary_operator(resolved_operator, &left_operand, &right_operand)?
                .with_subtype(None)
        } else if let Some(adjustment_operator) = execution_plan.relative_adjustment_operator {
            // Relative addition reads the right operand as a fraction of the
            // left operand, so `100 - 50%` evaluates its operands once and
            // combines `left - (left * scaled_right)`.
            let left_magnitude =
                self.execute_scale_plan(&left_operand, &execution_plan.left_operand_scale)?;
            let scaled_right =
                self.execute_scale_plan(&right_operand, &execution_plan.right_operand_scale)?;
            let adjustment_descriptor = self.redispatch_for_dynamic_types(
                self.registry.operator(adjustment_operator)?,
                &left_magnitude,
                &scaled_right,
            );
            let adjustment = self.execute_binary_operator(
                adjustment_descriptor,
                &left_magnitude,
                &scaled_right,
            )?;
            let outer_descriptor =
                self.redispatch_for_dynamic_types(resolved_operator, &left_magnitude, &adjustment);
            self.execute_binary_operator(outer_descriptor, &left_magnitude, &adjustment)?
                .with_subtype(resolution.output.subtype)
        } else {
            let left_operand =
                self.execute_scale_plan(&left_operand, &execution_plan.left_operand_scale)?;
            let right_operand =
                self.execute_scale_plan(&right_operand, &execution_plan.right_operand_scale)?;
            let resolved_operator =
                self.redispatch_for_dynamic_types(resolved_operator, &left_operand, &right_operand);
            self.execute_binary_operator(resolved_operator, &left_operand, &right_operand)?
                .with_subtype(resolution.output.subtype)
        };
        Ok(self
            .registry
            .transform_owned_configured_result(value, &self.configuration)?)
    }

    /// Resolves one genuinely open operator site and caches its prepared plan.
    fn open_binary_plan(
        &self,
        operator: BinaryOperator,
        dispatch: &TypedOpenBinaryDispatch,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<TypedBinaryPlan, RuntimeError> {
        let left_type = left_operand.scalar_type().ok_or_else(|| {
            RuntimeError::Message(format!("operator `{operator}` is not defined for an array"))
        })?;
        let right_type = right_operand.scalar_type().ok_or_else(|| {
            RuntimeError::Message(format!("operator `{operator}` is not defined for an array"))
        })?;
        let key = (left_type, right_type);
        if let Some(plan) = dispatch.get(key) {
            return Ok(plan);
        }
        let resolution = match operator {
            BinaryOperator::Addition | BinaryOperator::Subtraction => self
                .registry
                .resolve_subtype_relative_rule(operator, left_type, right_type)
                .or_else(|error| match error {
                    CoreError::SubtypeRelativeOperatorNotDefined { .. } => self
                        .registry
                        .resolve_binary_operation(operator, left_type, right_type),
                    other => Err(other),
                })?,
            _ => self
                .registry
                .resolve_binary_operation(operator, left_type, right_type)?,
        };
        let (left_operand_scale, scaled_left_type) =
            self.prepare_open_scale_plan(left_type.base, resolution.left_operand_scale)?;
        let right_scale = resolution
            .relative_adjustment
            .unwrap_or(resolution.right_operand_scale);
        let (right_operand_scale, scaled_right_type) =
            self.prepare_open_scale_plan(right_type.base, right_scale)?;
        let relative_adjustment_operator = resolution
            .relative_adjustment
            .map(|_| {
                self.registry.resolve_binary_operator(
                    BinaryOperator::Multiplication,
                    scaled_left_type,
                    scaled_right_type,
                )
            })
            .transpose()?;
        let plan = TypedBinaryPlan {
            resolution,
            execution_plan: TypedBinaryExecutionPlan {
                left_operand_scale,
                right_operand_scale,
                relative_adjustment_operator,
                result_domain: None,
            },
        };
        dispatch.insert(key, plan.clone());
        Ok(plan)
    }

    /// Resolves one genuinely open comparison site and caches its prepared plan.
    fn open_comparison_plan(
        &self,
        operator: ComparisonOperator,
        dispatch: &TypedOpenComparisonDispatch,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<TypedComparisonPlan, RuntimeError> {
        let left_type = left_operand.scalar_type().ok_or_else(|| {
            RuntimeError::Message(format!(
                "comparison `{operator}` is not defined for an array"
            ))
        })?;
        let right_type = right_operand.scalar_type().ok_or_else(|| {
            RuntimeError::Message(format!(
                "comparison `{operator}` is not defined for an array"
            ))
        })?;
        let key = (left_type, right_type);
        if let Some(plan) = dispatch.get(key) {
            return Ok(plan);
        }
        let resolution = self
            .registry
            .resolve_comparison_operation(operator, left_type, right_type)?;
        let left_operand_scale = self
            .prepare_open_scale_plan(left_type.base, resolution.left_operand_scale)?
            .0;
        let right_operand_scale = self
            .prepare_open_scale_plan(right_type.base, resolution.right_operand_scale)?
            .0;
        let plan = TypedComparisonPlan {
            resolution,
            left_operand_scale,
            right_operand_scale,
        };
        dispatch.insert(key, plan.clone());
        Ok(plan)
    }

    /// Builds the numeric scaling steps required by one open cache miss.
    fn prepare_open_scale_plan(
        &self,
        base_type: crate::semantic::TypeId,
        scale: Scale,
    ) -> Result<(TypedScalePlan, crate::semantic::TypeId), RuntimeError> {
        let mut scaled_type = base_type;
        let numerator = if scale.numerator == 1 {
            None
        } else {
            let factor = self
                .registry
                .parse_numeric(&scale.numerator.to_string(), Some(scaled_type))?;
            let operator = self.registry.resolve_binary_operator(
                BinaryOperator::Multiplication,
                scaled_type,
                factor.type_id(),
            )?;
            scaled_type = self.registry.operator(operator)?.result_type;
            Some(TypedScaleStep { operator, factor })
        };
        let denominator = if scale.denominator == 1 {
            None
        } else {
            let divisor_type = if self.registry.default_integer().ok() == Some(scaled_type) {
                self.registry.default_fractional()?
            } else {
                scaled_type
            };
            let factor = self
                .registry
                .parse_numeric(&scale.denominator.to_string(), Some(divisor_type))?;
            let operator = self.registry.resolve_binary_operator(
                BinaryOperator::Division,
                scaled_type,
                factor.type_id(),
            )?;
            scaled_type = self.registry.operator(operator)?.result_type;
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

    /// Selects the plan a dynamic operand pair's runtime subtypes call for.
    ///
    /// Each dynamic operand contributes one slot per candidate subtype, and the
    /// slot maps arithmetically to the plan the compiler resolved, so selecting
    /// a plan performs no type or subtype lookup. A missing plan means the
    /// registry defines no operation for that pair, which only fails when the
    /// program actually stores those subtypes.
    pub(super) fn select_binary_plan<'plan>(
        &self,
        operator: BinaryOperator,
        dispatch: &'plan crate::ir::TypedBinaryDispatch,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<&'plan crate::ir::TypedBinaryPlan, RuntimeError> {
        let left_slot = dispatch
            .left_domain
            .candidate_index(self.registry, left_operand.value_type());
        let right_slot = dispatch
            .right_domain
            .candidate_index(self.registry, right_operand.value_type());
        let index = match (left_slot, right_slot) {
            (Some(left_slot), Some(right_slot)) => {
                left_slot * dispatch.right_domain.len() + right_slot
            }
            _ => {
                return Err(RuntimeError::Message(format!(
                    "operator `{operator}` is not defined for `{}` and `{}`",
                    self.registry.value_type_name(left_operand.value_type()),
                    self.registry.value_type_name(right_operand.value_type())
                )));
            }
        };
        dispatch
            .plans
            .get(index)
            .and_then(|plan| plan.as_ref())
            .ok_or_else(|| {
                RuntimeError::Message(format!(
                    "operator `{operator}` is not defined for `{}` and `{}`",
                    self.registry.value_type_name(left_operand.value_type()),
                    self.registry.value_type_name(right_operand.value_type())
                ))
            })
    }

    /// Selects the relation a dynamic operand pair's runtime subtypes call for.
    pub(super) fn select_comparison_resolution<'dispatch>(
        &self,
        operator: ComparisonOperator,
        dispatch: &'dispatch crate::ir::TypedComparisonDispatch,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<&'dispatch crate::ir::TypedComparisonPlan, RuntimeError> {
        let left_slot = dispatch
            .left_domain
            .candidate_index(self.registry, left_operand.value_type());
        let right_slot = dispatch
            .right_domain
            .candidate_index(self.registry, right_operand.value_type());
        let index = match (left_slot, right_slot) {
            (Some(left_slot), Some(right_slot)) => {
                left_slot * dispatch.right_domain.len() + right_slot
            }
            _ => {
                return Err(RuntimeError::Message(format!(
                    "comparison `{operator}` is not defined for `{}` and `{}`",
                    self.registry.value_type_name(left_operand.value_type()),
                    self.registry.value_type_name(right_operand.value_type())
                )));
            }
        };
        dispatch
            .resolutions
            .get(index)
            .and_then(|resolution| resolution.as_ref())
            .ok_or_else(|| {
                RuntimeError::Message(format!(
                    "comparison `{operator}` is not defined for `{}` and `{}`",
                    self.registry.value_type_name(left_operand.value_type()),
                    self.registry.value_type_name(right_operand.value_type())
                ))
            })
    }

    /// Executes one compiler-resolved comparison relation.
    ///
    /// A dynamically typed operand can hold a representation the compiler did
    /// not predict, for example when a conversion promotes an integer magnitude
    /// to a fractional one. Re-resolving the relation for the actual operand
    /// pair mirrors the arithmetic path's dynamic-type redispatch, while the
    /// matching case keeps using the compiled executor without another lookup.
    pub(super) fn execute_compiled_comparison(
        &self,
        plan: &crate::ir::TypedComparisonPlan,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<Value, RuntimeError> {
        let left_operand = self.execute_scale_plan(left_operand, &plan.left_operand_scale)?;
        let right_operand = self.execute_scale_plan(right_operand, &plan.right_operand_scale)?;
        let descriptor = self.registry.comparison(plan.resolution.comparison)?;
        let execute = if left_operand.type_id() == descriptor.left_operand_type
            && right_operand.type_id() == descriptor.right_operand_type
        {
            descriptor.execute
        } else {
            self.registry
                .resolve_comparison_operation(
                    descriptor.operator,
                    left_operand.value_type(),
                    right_operand.value_type(),
                )
                .and_then(|resolved| self.registry.comparison(resolved.comparison))
                .map(|replacement| replacement.execute)
                .unwrap_or(descriptor.execute)
        };
        Ok(Value::new(
            plan.resolution.output.base,
            (execute)(&left_operand, &right_operand)?,
        ))
    }
}
