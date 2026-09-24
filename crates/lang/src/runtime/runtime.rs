use crate::ir::{
    LocalVariableSlot, TypedBlock, TypedExpression, TypedProgram, TypedRangePlan, TypedStatement,
};
use crate::semantic::{
    BinaryOperator, BinaryOperatorDescriptor, ComparisonExecutor, ComparisonOperator, Registry,
    ResolvedBinaryOperator, RuntimeConfiguration, Value, ValueType,
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

pub(crate) struct Runtime<'registry> {
    pub(crate) registry: &'registry Registry,
    pub(crate) configuration: RuntimeConfiguration,
    pub(crate) local_values: Vec<Option<Value>>,
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
                index_dispatch,
                expression,
                ..
            } => self.execute_indexed_assignment(
                *slot,
                index,
                *constant_index,
                *index_extractor,
                index_dispatch.as_ref(),
                expression,
            )?,
            TypedStatement::MapIndexedAssignment {
                slot,
                key,
                expression,
                ..
            } => {
                self.write_map_value(*slot, key, expression)?;
            }
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
        let result = (|| -> Result<(), RuntimeError> {
            let start_value = self
                .eval(start)?
                .ok_or_else(|| RuntimeError::Message("for range start returned no value".into()))?;
            let end_value = self
                .eval(end)?
                .ok_or_else(|| RuntimeError::Message("for range end returned no value".into()))?;
            self.require_integer_bound(&start_value)?;
            self.require_integer_bound(&end_value)?;
            let loop_body_plan = self.compile_loop_body_execution_plan(slot, body)?;
            let mut value_stack = Vec::with_capacity(loop_body_plan.value_stack_capacity);
            if self.execute_native_i64_range(
                slot,
                &start_value,
                &end_value,
                typed_range_plan,
                &loop_body_plan,
                &mut value_stack,
            )? {
                return Ok(());
            }
            let mut range_plan = self.activate_range_plan(typed_range_plan)?;
            let mut current = start_value;
            loop {
                if current.value_type() != range_plan.current_type {
                    range_plan = self.resolve_range_plan(&current, &end_value)?;
                }
                if !self.range_continues(&current, &end_value, &range_plan)? {
                    break;
                }
                if loop_body_plan.range_slot_is_used {
                    self.store_local_value(slot, current.clone());
                }
                self.execute_loop_body(&loop_body_plan, &mut value_stack)?;
                match self.loop_control.take() {
                    Some(LoopControl::Break) => break,
                    Some(LoopControl::Continue) | None => {}
                }
                current = self.advance_range_value(&current, &range_plan)?;
            }
            Ok(())
        })();
        self.clear_local_slots(std::slice::from_ref(&slot));
        result
    }

    /// Executes the ordinary default-integer range without generic value dispatch.
    fn execute_native_i64_range(
        &mut self,
        slot: LocalVariableSlot,
        start: &Value,
        end: &Value,
        typed_range_plan: &TypedRangePlan,
        loop_body_plan: &loop_execution::LoopBodyExecutionPlan<'_>,
        value_stack: &mut Vec<Value>,
    ) -> Result<bool, RuntimeError> {
        let integer = self.registry.default_integer()?;
        let integer_type = ValueType::plain(integer);
        if start.value_type() != integer_type
            || end.value_type() != integer_type
            || typed_range_plan.current_type != integer_type
            || loop_body_plan.changes_configuration
            || !self.configuration.uses_initial_values()
            || !self
                .registry
                .initial_result_transform_is_identity(integer)?
        {
            return Ok(false);
        }
        let (Some(mut current), Some(end)) = (
            start.downcast_ref::<i64>().copied(),
            end.downcast_ref::<i64>().copied(),
        ) else {
            return Ok(false);
        };
        while current < end {
            if loop_body_plan.range_slot_is_used {
                self.store_local_value(slot, Value::new(integer, current));
            }
            self.execute_loop_body(loop_body_plan, value_stack)?;
            match self.loop_control.take() {
                Some(LoopControl::Break) => break,
                Some(LoopControl::Continue) | None => {}
            }
            current += 1;
        }
        Ok(true)
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
        let result = (|| -> Result<(), RuntimeError> {
            for statement in &block.statements {
                self.execute_statement(statement)?;
                if self.loop_control.is_some() {
                    break;
                }
            }
            Ok(())
        })();
        self.clear_local_slots(&block.owned_slots);
        result
    }

    /// Drops every value owned directly by one lexical scope.
    fn clear_local_slots(&mut self, slots: &[LocalVariableSlot]) {
        for slot in slots {
            self.local_values[slot.0].take();
        }
    }

    /// Stores a local value in its compiler-assigned slot.
    fn store_local_value(&mut self, slot: LocalVariableSlot, value: Value) {
        self.local_values[slot.0] = Some(value);
    }
}

#[cfg(test)]
#[path = "runtime.tests.rs"]
mod tests;
