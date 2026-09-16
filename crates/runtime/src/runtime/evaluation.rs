use ir::{TypedBinaryExecutionPlan, TypedExpression, TypedExpressionKind, TypedScalePlan};
use language_core::{
    ArrayValue, BinaryOperator, BinaryOperatorDescriptor, ComparisonOperator, CoreError,
    ExecutionContext, Registry, ResolvedBinaryOperator, ResolvedComparison, Scale, Value,
};
use syntax::LogicalOperator;

use super::Runtime;
use crate::RuntimeError;

impl<'registry> Runtime<'registry> {
    pub(super) fn eval(
        &mut self,
        expression: &TypedExpression,
    ) -> Result<Option<Value>, RuntimeError> {
        match &expression.kind {
            TypedExpressionKind::Literal(value) => Ok(Some(value.clone())),
            TypedExpressionKind::Variable { name, slot, .. } => self.local_values[slot.0]
                .clone()
                .map(Some)
                .ok_or_else(|| RuntimeError::Message(format!("unknown runtime variable `{name}`"))),
            TypedExpressionKind::Binary {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
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
            TypedExpressionKind::Comparison {
                resolution,
                left_operand,
                right_operand,
            } => {
                let left_operand = self.eval(left_operand)?.ok_or_else(|| {
                    RuntimeError::Message("left comparison operand returned no value".into())
                })?;
                let right_operand = self.eval(right_operand)?.ok_or_else(|| {
                    RuntimeError::Message("right comparison operand returned no value".into())
                })?;
                let value =
                    self.execute_compiled_comparison(resolution, &left_operand, &right_operand)?;
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
                    self.execute_compiled_comparison(&resolution, &left_operand, &right_operand)?;
                Ok(Some(value))
            }
            TypedExpressionKind::NullCheck {
                operand,
                equal,
                boolean_type,
            } => {
                let value = self.eval(operand)?.ok_or_else(|| {
                    RuntimeError::Message("null-check operand returned no value".into())
                })?;
                let is_null = value.type_id() == self.registry.default_null()?;
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
                expression,
            } => {
                let value = self.eval(expression)?.ok_or_else(|| {
                    RuntimeError::Message("converted expression returned no value".into())
                })?;
                let scaled = self
                    .scale_magnitude(&value, conversion.scale)?
                    .with_subtype(None);
                let cast = match target_base {
                    Some(target) => self.cast_base(&scaled, *target)?,
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
                let slot = self.registry.subtype_dispatch_slot(value.subtype_id());
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
                let scaled = self
                    .scale_magnitude(&value, plan.conversion.scale)?
                    .with_subtype(None);
                let cast = match plan.target_base {
                    Some(target) => self.cast_base(&scaled, target)?,
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
            TypedExpressionKind::ArrayLiteral {
                array_type,
                elements,
            } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    values.push(self.eval(element)?.ok_or_else(|| {
                        RuntimeError::Message("array element returned no value".into())
                    })?);
                }
                Ok(Some(Value::new(
                    array_type.element.base,
                    ArrayValue::new(values),
                )))
            }
            TypedExpressionKind::ElementAccess {
                array,
                index,
                constant_index,
                index_extractor,
                ..
            } => {
                let array_value = self.eval(array)?.ok_or_else(|| {
                    RuntimeError::Message("array expression returned no value".into())
                })?;
                let elements = array_value
                    .downcast_ref::<ArrayValue>()
                    .ok_or_else(|| RuntimeError::Message("value is not an array".into()))?;
                let index = match constant_index {
                    Some(index) => *index,
                    None => {
                        let index_value = self.eval(index)?.ok_or_else(|| {
                            RuntimeError::Message("array index returned no value".into())
                        })?;
                        self.require_plain_index(&index_value)?;
                        index_extractor(&index_value)?
                    }
                };
                let element = elements.elements().get(index).ok_or_else(|| {
                    RuntimeError::Message(format!(
                        "array index {index} is out of bounds for length {}",
                        elements.elements().len()
                    ))
                })?;
                Ok(Some(element.clone()))
            }
        }
    }

    /// Applies one resolved subtype scale to a scalar runtime value.
    fn scale_magnitude(&self, value: &Value, scale: Scale) -> Result<Value, RuntimeError> {
        let mut scaled = value.clone().with_subtype(None);
        if scale.is_identity() {
            return Ok(scaled);
        }

        if scale.numerator != 1 {
            let factor = self
                .registry
                .parse_numeric(&scale.numerator.to_string(), Some(scaled.type_id()))?;
            let operator = self.registry.resolve_binary_operator(
                BinaryOperator::Multiplication,
                scaled.type_id(),
                factor.type_id(),
            )?;
            let descriptor = self.registry.operator(operator)?;
            scaled = self.execute_binary_operator(descriptor, &scaled, &factor)?;
        }

        if scale.denominator != 1 {
            let divisor_type = if self.registry.default_integer().ok() == Some(scaled.type_id()) {
                self.registry.default_fractional()?
            } else {
                scaled.type_id()
            };
            let divisor = self
                .registry
                .parse_numeric(&scale.denominator.to_string(), Some(divisor_type))?;
            let operator = self.registry.resolve_binary_operator(
                BinaryOperator::Division,
                scaled.type_id(),
                divisor.type_id(),
            )?;
            let descriptor = self.registry.operator(operator)?;
            scaled = self.execute_binary_operator(descriptor, &scaled, &divisor)?;
        }

        Ok(scaled)
    }

    /// Applies a compiler-prepared magnitude scale without parsing or signature lookup.
    fn execute_scale_plan(
        &self,
        value: &Value,
        scale_plan: &TypedScalePlan,
    ) -> Result<Value, RuntimeError> {
        let mut scaled = value.clone().with_subtype(None);
        for step in [
            scale_plan.numerator.as_ref(),
            scale_plan.denominator.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
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

    /// Casts one plain numeric value to the requested base representation.
    ///
    /// The conversion round-trips through the source formatter and the target
    /// numeric parser so no concrete-type branch is needed in the runtime.
    /// Integer-valued magnitudes such as `1500.0` are accepted for integer
    /// targets by retrying with the whole part when it carries only zeros.
    fn cast_base(
        &self,
        value: &Value,
        target: language_core::TypeId,
    ) -> Result<Value, RuntimeError> {
        if value.type_id() == target {
            return Ok(value.clone());
        }
        let formatted = (self.registry.type_descriptor(value.type_id())?.format)(value)?;
        match self.registry.parse_numeric(&formatted, Some(target)) {
            Ok(cast) => Ok(cast),
            Err(_) => {
                if let Some((whole, fractional)) = formatted.split_once('.')
                    && fractional.trim_end_matches('0').is_empty()
                {
                    let whole = if whole.is_empty() || whole == "-" || whole == "+" {
                        format!("{whole}0")
                    } else {
                        whole.to_string()
                    };
                    if let Ok(cast) = self.registry.parse_numeric(&whole, Some(target)) {
                        return Ok(cast);
                    }
                }
                Err(RuntimeError::from(language_core::CoreError::Runtime(
                    format!(
                        "cannot convert `{}` to `{}`",
                        self.registry.value_type_name(value.value_type()),
                        self.registry.type_name(target)
                    ),
                )))
            }
        }
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
        dispatch: &'plan ir::TypedBinaryDispatch,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<&'plan ir::TypedBinaryPlan, RuntimeError> {
        let index = dynamic_dispatch_index(
            self.registry,
            dispatch.left_width,
            left_operand,
            dispatch.right_width,
            right_operand,
        );
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
    pub(super) fn select_comparison_resolution(
        &self,
        operator: ComparisonOperator,
        dispatch: &ir::TypedComparisonDispatch,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<ResolvedComparison, RuntimeError> {
        let index = dynamic_dispatch_index(
            self.registry,
            dispatch.left_width,
            left_operand,
            dispatch.right_width,
            right_operand,
        );
        dispatch
            .resolutions
            .get(index)
            .and_then(|resolution| resolution.as_ref())
            .copied()
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
        resolution: &ResolvedComparison,
        left_operand: &Value,
        right_operand: &Value,
    ) -> Result<Value, RuntimeError> {
        let left_operand = self.scale_magnitude(left_operand, resolution.left_operand_scale)?;
        let right_operand = self.scale_magnitude(right_operand, resolution.right_operand_scale)?;
        let descriptor = self.registry.comparison(resolution.comparison)?;
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
            resolution.output.base,
            (execute)(&left_operand, &right_operand)?,
        ))
    }
}

/// Returns the row-major dispatch index for one operand pair.
///
/// A width of one means the operand's complete type is static, so it always
/// addresses slot zero. Every other width means the value's own subtype selects
/// the slot the compiler resolved for that candidate subtype.
fn dynamic_dispatch_index(
    registry: &Registry,
    left_width: usize,
    left_operand: &Value,
    right_width: usize,
    right_operand: &Value,
) -> usize {
    let left_slot = if left_width == 1 {
        0
    } else {
        registry.subtype_dispatch_slot(left_operand.subtype_id())
    };
    let right_slot = if right_width == 1 {
        0
    } else {
        registry.subtype_dispatch_slot(right_operand.subtype_id())
    };
    left_slot * right_width + right_slot
}
