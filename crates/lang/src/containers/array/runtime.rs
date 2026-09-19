//! Runtime execution of array expressions and the built-in array operations.
//!
//! The evaluator matches an array expression and delegates here, so every
//! language-level array step lives with the array module: building a literal,
//! crossing the element contract, reading and writing one element, and applying
//! an end operation. The payload itself owns the element movement, the bounds
//! contract, and the copy-on-write decision.

use crate::containers::array::{
    ArrayValue, apply_element_contract, apply_end_operation, element_at, set_element,
};
use crate::ir::{LocalVariableSlot, TypedExpression, TypedIndexDispatch};
use crate::semantic::{ArrayEndOperation, IndexExtractor, Value, ValueType};

use crate::RuntimeError;
use crate::runtime::Runtime;

impl Runtime<'_> {
    /// Builds the array value one literal produces.
    ///
    /// Every element is evaluated in source order before the payload exists, so a
    /// failing element expression never produces a partially filled array.
    pub(crate) fn build_array_value(
        &mut self,
        expression: &TypedExpression,
        elements: &[TypedExpression],
    ) -> Result<Option<Value>, RuntimeError> {
        let array_type = expression.array_type().ok_or_else(|| {
            RuntimeError::Message("array literal does not have an array semantic output".into())
        })?;
        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            values.push(
                self.eval(element)?.ok_or_else(|| {
                    RuntimeError::Message("array element returned no value".into())
                })?,
            );
        }
        Ok(Some(Value::new_array(array_type, ArrayValue::new(values))))
    }

    /// Applies the declared element contract to a value entering storage.
    ///
    /// The element expression is evaluated first and the contract is applied to
    /// the value it produced, so a rejected store never touches the array.
    pub(crate) fn store_element_value(
        &mut self,
        element: ValueType,
        expression: &TypedExpression,
    ) -> Result<Option<Value>, RuntimeError> {
        let value = self
            .eval(expression)?
            .ok_or_else(|| RuntimeError::Message("array element returned no value".into()))?;
        Ok(Some(apply_element_contract(
            self.registry,
            &self.configuration,
            value,
            element,
        )?))
    }

    /// Reads the element one compiled index plan selects.
    pub(crate) fn read_array_element(
        &mut self,
        array: &TypedExpression,
        index: &TypedExpression,
        constant_index: Option<usize>,
        index_extractor: IndexExtractor,
        index_dispatch: Option<&TypedIndexDispatch>,
    ) -> Result<Option<Value>, RuntimeError> {
        let array_value = self
            .eval(array)?
            .ok_or_else(|| RuntimeError::Message("array expression returned no value".into()))?;
        let index =
            self.resolve_element_index(index, constant_index, index_extractor, index_dispatch)?;
        Ok(Some(element_at(&array_value, index)?))
    }

    /// Applies one built-in end operation to the array stored in `slot`.
    ///
    /// The argument an addition stores is evaluated before the array is touched,
    /// so a failing argument expression leaves the array unchanged. The payload
    /// owns the element movement, the copy-on-write decision, and the empty-array
    /// contract, so this keeps only the language-level parts: evaluating the
    /// argument, reading the binding, and producing the value the operation
    /// reports.
    ///
    /// A removal from an empty array produces the language's null value rather
    /// than reading an element that does not exist, and an addition produces no
    /// value at all. The null value is the one the compiler resolved for this
    /// call, so an empty removal repeats neither a type lookup nor a literal
    /// parse during execution.
    pub(crate) fn execute_array_method(
        &mut self,
        method: ArrayEndOperation,
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
        let array_value = self.local_values[slot.0]
            .as_mut()
            .ok_or_else(|| RuntimeError::Message("array binding is not initialized".into()))?;
        if array_value.array_type().is_none() {
            return Err(RuntimeError::Message("value is not an array".into()));
        }
        match apply_end_operation(array_value, method, stored_value)? {
            Some(value) => Ok(Some(value)),
            None => Ok(empty_result.clone()),
        }
    }

    /// Replaces one element of the mutable array stored in `slot`.
    ///
    /// The element is evaluated before the array is touched, so a failing element
    /// expression leaves the array unchanged. The payload owns the bounds contract
    /// and the copy-on-write decision, which preserves the value semantics of an
    /// array shared with another binding instead of aliasing the change into the
    /// other binding.
    pub(crate) fn execute_indexed_assignment(
        &mut self,
        slot: LocalVariableSlot,
        index: &TypedExpression,
        constant_index: Option<usize>,
        index_extractor: IndexExtractor,
        index_dispatch: Option<&TypedIndexDispatch>,
        expression: &TypedExpression,
    ) -> Result<(), RuntimeError> {
        let element = self.eval(expression)?.ok_or_else(|| {
            RuntimeError::Message("array element assignment returned no value".into())
        })?;
        let index =
            self.resolve_element_index(index, constant_index, index_extractor, index_dispatch)?;
        let array_value = self.local_values[slot.0]
            .as_mut()
            .ok_or_else(|| RuntimeError::Message("array binding is not initialized".into()))?;
        if array_value.array_type().is_none() {
            return Err(RuntimeError::Message("value is not an array".into()));
        }
        set_element(array_value, index, element)?;
        Ok(())
    }

    /// Resolves the element index one read or one write uses.
    ///
    /// A constant index is already resolved by the compiler. Otherwise the index
    /// expression is evaluated and one pre-resolved extractor converts the value,
    /// so the runtime never resolves a type by name. A read and a write both
    /// resolve their index here, so a rejected index is reported identically on
    /// both paths.
    fn resolve_element_index(
        &mut self,
        index: &TypedExpression,
        constant_index: Option<usize>,
        index_extractor: IndexExtractor,
        index_dispatch: Option<&TypedIndexDispatch>,
    ) -> Result<usize, RuntimeError> {
        let Some(constant) = constant_index else {
            let index_value = self
                .eval(index)?
                .ok_or_else(|| RuntimeError::Message("array index returned no value".into()))?;
            self.require_plain_index(&index_value)?;
            return Ok(match index_dispatch {
                Some(dispatch) => {
                    let slot = dispatch
                        .domain
                        .candidate_index(self.registry, index_value.value_type())
                        .ok_or_else(|| self.invalid_index_error(&index_value))?;
                    dispatch
                        .extractors
                        .get(slot)
                        .and_then(|extractor| *extractor)
                        .ok_or_else(|| self.invalid_index_error(&index_value))?(
                        &index_value
                    )?
                }
                None => index_extractor(&index_value)?,
            });
        };
        Ok(constant)
    }

    /// Requires a dynamically computed index to be an unqualified integer.
    ///
    /// The compiler rejects a statically qualified index because the element
    /// index is a position, not a magnitude. A dynamically typed index cannot be
    /// checked at compile time, so the same rule is enforced here, keeping the
    /// two paths consistent instead of silently reading a qualified magnitude.
    fn require_plain_index(&self, index: &Value) -> Result<(), RuntimeError> {
        if index.subtype_id().is_some() {
            return Err(RuntimeError::Message(format!(
                "an array index must be a plain integer, found `{}`",
                self.registry.value_type_name(index.value_type())
            )));
        }
        Ok(())
    }

    /// Reports a value this type cannot use as an array index.
    fn invalid_index_error(&self, index: &Value) -> RuntimeError {
        RuntimeError::Message(format!(
            "type `{}` cannot be used as an array index",
            self.registry.type_name(index.type_id())
        ))
    }
}
