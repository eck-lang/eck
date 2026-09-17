use ir::{
    ArrayMethod, LocalVariableSlot, TypedBlock, TypedExpression, TypedProgram, TypedRangePlan,
    TypedStatement,
};
use language_core::{
    ArrayValue, BinaryOperator, BinaryOperatorDescriptor, ComparisonExecutor, ComparisonOperator,
    IndexExtractor, Registry, ResolvedBinaryOperator, RuntimeConfiguration, Value, ValueType,
};

use crate::RuntimeError;

mod evaluation;
mod loop_execution;

pub fn execute(program: &TypedProgram, registry: &Registry) -> Result<(), RuntimeError> {
    let mut runtime = Runtime {
        registry,
        configuration: registry.default_runtime_configuration(),
        local_values: vec![None; program.local_slot_count],
        loop_control: None,
    };
    for statement in &program.statements {
        runtime.execute_statement(statement)?;
    }
    Ok(())
}

struct Runtime<'registry> {
    registry: &'registry Registry,
    configuration: RuntimeConfiguration,
    local_values: Vec<Option<Value>>,
    loop_control: Option<LoopControl>,
}

/// Records a control transfer requested by the currently executing loop body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoopControl {
    Break,
    Continue,
}

/// Caches one range's comparison and increment dispatch for its current type.
///
/// Fixed-width integer overflows can promote the iteration value at runtime.
/// When that happens, the runtime rebuilds this cache once for the widened
/// type; otherwise every iteration uses direct function pointers and values.
struct RangeExecutionPlan {
    current_type: ValueType,
    increment_unit: Value,
    comparison_execute: ComparisonExecutor,
    increment: ResolvedBinaryOperator,
    increment_descriptor: BinaryOperatorDescriptor,
}

impl<'registry> Runtime<'registry> {
    fn execute_statement(&mut self, statement: &TypedStatement) -> Result<(), RuntimeError> {
        match statement {
            TypedStatement::Configuration {
                configuration_override,
                ..
            } => {
                self.configuration.apply(configuration_override);
            }
            TypedStatement::VariableDeclaration {
                slot, expression, ..
            } => {
                let value = self
                    .eval(expression)?
                    .ok_or_else(|| RuntimeError::Message("initializer returned no value".into()))?;
                self.store_local_value(*slot, value);
            }
            TypedStatement::Assignment {
                slot, expression, ..
            } => {
                let value = self
                    .eval(expression)?
                    .ok_or_else(|| RuntimeError::Message("assignment returned no value".into()))?;
                self.store_local_value(*slot, value);
            }
            TypedStatement::IndexedAssignment {
                slot,
                index,
                constant_index,
                index_extractor,
                expression,
                ..
            } => self.execute_indexed_assignment(
                *slot,
                index,
                *constant_index,
                *index_extractor,
                expression,
            )?,
            TypedStatement::Block(block) => self.execute_block(block)?,
            TypedStatement::If {
                condition,
                body,
                else_body,
                ..
            } => {
                let condition = self.eval(condition)?.ok_or_else(|| {
                    RuntimeError::Message("if condition returned no value".into())
                })?;
                if self.registry.evaluate_boolean(&condition)? {
                    self.execute_block(body)?;
                } else if let Some(else_body) = else_body {
                    self.execute_block(else_body)?;
                }
            }
            TypedStatement::While {
                condition, body, ..
            } => loop {
                let condition = self.eval(condition)?.ok_or_else(|| {
                    RuntimeError::Message("while condition returned no value".into())
                })?;
                if !self.registry.evaluate_boolean(&condition)? {
                    break;
                }
                self.execute_block(body)?;
                match self.loop_control.take() {
                    Some(LoopControl::Break) => break,
                    Some(LoopControl::Continue) | None => {}
                }
            },
            TypedStatement::Expression(expression) => {
                let _ = self.eval(expression)?;
            }
            TypedStatement::For {
                slot,
                start,
                end,
                range_plan,
                body,
                ..
            } => self.execute_for(*slot, start, end, range_plan, body)?,
            TypedStatement::Break { .. } => self.loop_control = Some(LoopControl::Break),
            TypedStatement::Continue { .. } => self.loop_control = Some(LoopControl::Continue),
        }
        Ok(())
    }

    /// Applies one built-in end operation to the array stored in `slot`.
    ///
    /// Every argument is evaluated before the array is touched, so a failing
    /// argument expression leaves the array unchanged. The payload is mutated in
    /// place while it is uniquely owned and copied on write otherwise, which
    /// preserves the value semantics of an array shared with another binding
    /// instead of aliasing the change into the other binding. The operation
    /// itself moves only the element it adds or removes: the buffer claims a
    /// free slot at the required end, or releases the element at that end.
    ///
    /// A removal from an empty array produces the language's null value rather
    /// than reading an element that does not exist, and an addition produces no
    /// value at all. The null value is the one the compiler resolved for this
    /// call, so an empty removal repeats neither a type lookup nor a literal
    /// parse during execution.
    fn execute_array_method(
        &mut self,
        method: ArrayMethod,
        slot: LocalVariableSlot,
        arguments: &[TypedExpression],
        empty_result: &Option<Value>,
    ) -> Result<Option<Value>, RuntimeError> {
        let stored_value =
            match method.removes_element() {
                false => Some(self.eval(&arguments[0])?.ok_or_else(|| {
                    RuntimeError::Message("array element returned no value".into())
                })?),
                true => None,
            };
        let mut array_value = self.local_values[slot.0]
            .take()
            .ok_or_else(|| RuntimeError::Message("array binding is not initialized".into()))?;
        if array_value.downcast_ref::<ArrayValue>().is_none() {
            self.store_local_value(slot, array_value);
            return Err(RuntimeError::Message("value is not an array".into()));
        }
        if !array_value.is_uniquely_owned() {
            let array_type = array_value.type_id();
            let copied = array_value
                .downcast_ref::<ArrayValue>()
                .expect("the value was verified as an array above")
                .clone();
            array_value = Value::new(array_type, copied);
        }
        let array = array_value
            .downcast_mut::<ArrayValue>()
            .expect("the array payload is uniquely owned here");
        let removed = apply_array_method(array, method, stored_value);
        self.store_local_value(slot, array_value);
        match removed {
            Some(value) => Ok(Some(value)),
            None => Ok(empty_result.clone()),
        }
    }

    /// Replaces one element of the mutable array stored in `slot`.
    ///
    /// The element is evaluated before the array is touched, so a failing
    /// element expression leaves the array unchanged. The array payload is
    /// mutated in place while it is uniquely owned and copied on write
    /// otherwise, which preserves the value semantics of an array shared with
    /// another binding instead of aliasing the change into the other binding.
    fn execute_indexed_assignment(
        &mut self,
        slot: LocalVariableSlot,
        index: &TypedExpression,
        constant_index: Option<usize>,
        index_extractor: IndexExtractor,
        expression: &TypedExpression,
    ) -> Result<(), RuntimeError> {
        let element = self.eval(expression)?.ok_or_else(|| {
            RuntimeError::Message("array element assignment returned no value".into())
        })?;
        let index = match constant_index {
            Some(index) => index,
            None => {
                let index_value = self
                    .eval(index)?
                    .ok_or_else(|| RuntimeError::Message("array index returned no value".into()))?;
                self.require_plain_index(&index_value)?;
                index_extractor(&index_value)?
            }
        };
        let mut array_value = self.local_values[slot.0]
            .take()
            .ok_or_else(|| RuntimeError::Message("array binding is not initialized".into()))?;
        let length = array_value
            .downcast_ref::<ArrayValue>()
            .map(|array| array.elements().len())
            .ok_or_else(|| RuntimeError::Message("value is not an array".into()))?;
        if index >= length {
            self.store_local_value(slot, array_value);
            return Err(RuntimeError::Message(format!(
                "array index {index} is out of bounds for length {length}"
            )));
        }
        if let Some(array) = array_value.downcast_mut::<ArrayValue>() {
            array.elements_mut()[index] = element;
        } else {
            let array_type = array_value.type_id();
            let mut elements = array_value
                .downcast_ref::<ArrayValue>()
                .expect("the value was verified as an array above")
                .elements()
                .to_vec();
            elements[index] = element;
            array_value = Value::new(array_type, ArrayValue::new(elements));
        }
        self.store_local_value(slot, array_value);
        Ok(())
    }

    /// Executes a `for (variable in start..end)` integer range loop.
    ///
    /// Both bounds evaluate exactly once before iteration begins. The loop
    /// owns a statically allocated local slot, so the source name never needs
    /// runtime resolution. Its compiled plan avoids registry resolution
    /// and unit parsing in the ordinary iteration hot path, while a widened
    /// value rebuilds the plan once through the same registry contracts.
    fn execute_for<'program>(
        &mut self,
        slot: LocalVariableSlot,
        start: &'program TypedExpression,
        end: &'program TypedExpression,
        typed_range_plan: &'program TypedRangePlan,
        body: &'program TypedBlock,
    ) -> Result<(), RuntimeError> {
        let start_value = self
            .eval(start)?
            .ok_or_else(|| RuntimeError::Message("for range start returned no value".into()))?;
        let end_value = self
            .eval(end)?
            .ok_or_else(|| RuntimeError::Message("for range end returned no value".into()))?;
        self.require_integer_bound(&start_value)?;
        self.require_integer_bound(&end_value)?;
        let mut range_plan = self.activate_range_plan(typed_range_plan)?;
        let loop_body_plan = self.compile_loop_body_execution_plan(slot, body)?;
        let mut value_stack = Vec::with_capacity(loop_body_plan.value_stack_capacity);
        let mut current = start_value;
        loop {
            if current.value_type() != range_plan.current_type {
                range_plan = self.resolve_range_plan(&current, &end_value)?;
            }
            if !self.range_continues(&current, &end_value, &range_plan)? {
                break;
            }
            self.store_local_value(slot, current.clone());
            self.execute_loop_body(&loop_body_plan, &mut value_stack)?;
            match self.loop_control.take() {
                Some(LoopControl::Break) => break,
                Some(LoopControl::Continue) | None => {}
            }
            current = self.advance_range_value(&current, &range_plan)?;
        }
        Ok(())
    }

    /// Validates that a range bound is an unqualified value of an integral type.
    ///
    /// Typed programs normally receive this validation from the compiler, but
    /// retaining it here keeps manually assembled IR from bypassing the range
    /// contract. It runs once per loop, never in the iteration hot path.
    fn require_integer_bound(&self, bound: &Value) -> Result<(), RuntimeError> {
        if bound.subtype_id().is_none() && self.registry.is_integer_type(bound.type_id())? {
            Ok(())
        } else {
            Err(RuntimeError::Message(format!(
                "for range bounds must be integers, found `{}`",
                self.registry.value_type_name(bound.value_type())
            )))
        }
    }

    /// Converts one compiler-resolved range plan into its direct runtime dispatch.
    fn activate_range_plan(
        &self,
        typed_range_plan: &TypedRangePlan,
    ) -> Result<RangeExecutionPlan, RuntimeError> {
        let comparison_execute = self
            .registry
            .comparison(typed_range_plan.comparison.comparison)?
            .execute;
        let increment_descriptor = self
            .registry
            .operator(typed_range_plan.increment.operator)?
            .clone();
        Ok(RangeExecutionPlan {
            current_type: typed_range_plan.current_type,
            increment_unit: typed_range_plan.increment_unit.clone(),
            comparison_execute,
            increment: typed_range_plan.increment,
            increment_descriptor,
        })
    }

    /// Resolves a replacement plan after an iteration value widens at runtime.
    fn resolve_range_plan(
        &self,
        current: &Value,
        end: &Value,
    ) -> Result<RangeExecutionPlan, RuntimeError> {
        let increment_unit = self.registry.parse_numeric("1", Some(current.type_id()))?;
        let comparison = self.registry.resolve_comparison_operation(
            ComparisonOperator::Less,
            current.value_type(),
            end.value_type(),
        )?;
        let increment = self.registry.resolve_binary_operation(
            BinaryOperator::Addition,
            current.value_type(),
            increment_unit.value_type(),
        )?;
        let comparison_execute = self.registry.comparison(comparison.comparison)?.execute;
        let increment_descriptor = self.registry.operator(increment.operator)?.clone();
        Ok(RangeExecutionPlan {
            current_type: current.value_type(),
            increment_unit,
            comparison_execute,
            increment,
            increment_descriptor,
        })
    }

    /// Evaluates `current < end` through the active range plan.
    ///
    /// Range bounds are always plain integers, so comparison resolution has
    /// identity scales and can call the cached executor without cloning or
    /// dynamically scaling either operand.
    fn range_continues(
        &self,
        current: &Value,
        end: &Value,
        range_plan: &RangeExecutionPlan,
    ) -> Result<bool, RuntimeError> {
        Ok((range_plan.comparison_execute)(current, end)?)
    }

    /// Computes `current + 1` through the active range plan.
    ///
    /// The compiler only creates range plans for plain integers, which makes
    /// both increment operand scales identities and keeps this hot path free
    /// of registry lookup, parsing, and operand cloning.
    fn advance_range_value(
        &self,
        current: &Value,
        range_plan: &RangeExecutionPlan,
    ) -> Result<Value, RuntimeError> {
        let value = self
            .execute_binary_operator(
                &range_plan.increment_descriptor,
                current,
                &range_plan.increment_unit,
            )?
            .with_subtype(range_plan.increment.output.subtype);
        self.registry
            .transform_owned_configured_result(value, &self.configuration)
            .map_err(RuntimeError::from)
    }

    /// Executes a block whose lexical variable bindings are already represented by slots.
    fn execute_block(&mut self, block: &TypedBlock) -> Result<(), RuntimeError> {
        for statement in &block.statements {
            self.execute_statement(statement)?;
            if self.loop_control.is_some() {
                break;
            }
        }
        Ok(())
    }

    /// Stores a local value in its compiler-assigned slot.
    fn store_local_value(&mut self, slot: LocalVariableSlot, value: Value) {
        self.local_values[slot.0] = Some(value);
    }

    /// Requires a dynamically computed index to be an unqualified integer.
    ///
    /// The compiler rejects a statically qualified index because the element
    /// index is a position, not a magnitude. A dynamically typed index cannot be
    /// checked at compile time, so the same rule is enforced here, keeping the
    /// two paths consistent instead of silently reading a qualified magnitude.
    pub(super) fn require_plain_index(&self, index: &Value) -> Result<(), RuntimeError> {
        if index.subtype_id().is_some() {
            return Err(RuntimeError::Message(format!(
                "an array index must be a plain integer, found `{}`",
                self.registry.value_type_name(index.value_type())
            )));
        }
        Ok(())
    }
}

/// Applies one built-in array end operation and reports the value it removed.
///
/// An adding operation stores the value it was given and produces nothing, while
/// a removing operation reports the element it moved out of the array or `None`
/// when the array was empty. The array buffer owns the movement of elements, so
/// this function only selects the end the operation acts on.
fn apply_array_method(
    array: &mut ArrayValue,
    method: ArrayMethod,
    stored_value: Option<Value>,
) -> Option<Value> {
    match method {
        ArrayMethod::Push => {
            array.push(stored_value.expect("an adding operation has one value to store"));
            None
        }
        ArrayMethod::Unshift => {
            array.unshift(stored_value.expect("an adding operation has one value to store"));
            None
        }
        ArrayMethod::Pop => array.pop(),
        ArrayMethod::Shift => array.shift(),
    }
}

#[cfg(test)]
#[path = "runtime.tests.rs"]
mod tests;
