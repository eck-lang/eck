//! Base-type binary operator registration and dispatch.

use crate::{
    BinaryOperator, BinaryOperatorDescriptor, BinaryOperatorExecutor, CoreError, OperatorId, TypeId,
};

use super::Registry;

impl Registry {
    /// Registers one binary operator implementation for an exact pair of base types.
    ///
    /// The returned ID is dense and remains stable for the lifetime of this
    /// registry, allowing compiled programs to dispatch directly at runtime.
    /// All input and result type IDs must already be registered. Returns
    /// [`CoreError::UnknownTypeId`] for an unknown type or
    /// [`CoreError::DuplicateOperator`] for an existing signature.
    pub fn register_binary_operator(
        &mut self,
        operator: BinaryOperator,
        left_operand_type: TypeId,
        right_operand_type: TypeId,
        result_type: TypeId,
        execute: BinaryOperatorExecutor,
    ) -> Result<OperatorId, CoreError> {
        self.type_descriptor(left_operand_type)?;
        self.type_descriptor(right_operand_type)?;
        self.type_descriptor(result_type)?;

        let key = (operator, left_operand_type, right_operand_type);
        if self.operator_index.contains_key(&key) {
            return Err(CoreError::DuplicateOperator {
                operator,
                left_operand_type: self.type_name(left_operand_type).to_string(),
                right_operand_type: self.type_name(right_operand_type).to_string(),
            });
        }

        let id = OperatorId {
            registry_id: self.registry_id,
            index: self.operators.len(),
        };
        self.operators.push(BinaryOperatorDescriptor {
            id,
            operator,
            left_operand_type,
            right_operand_type,
            result_type,
            execute,
        });
        self.operator_index.insert(key, id);
        Ok(id)
    }

    /// Resolves a binary operator for an exact pair of base types.
    ///
    /// This is the hot compile-time lookup used before subtype rules are
    /// considered. It does not search for implicit coercions.
    pub fn resolve_binary_operator(
        &self,
        operator: BinaryOperator,
        left_operand_type: TypeId,
        right_operand_type: TypeId,
    ) -> Result<OperatorId, CoreError> {
        self.operator_index
            .get(&(operator, left_operand_type, right_operand_type))
            .copied()
            .ok_or_else(|| CoreError::OperatorNotDefined {
                operator,
                left_operand_type: self.type_name(left_operand_type).to_string(),
                right_operand_type: self.type_name(right_operand_type).to_string(),
            })
    }

    /// Returns the executable descriptor identified by a previously resolved ID.
    ///
    /// IDs index a vector rather than requiring a second hash lookup during
    /// runtime evaluation. Returns [`CoreError::UnknownOperatorId`] when `id`
    /// was resolved by another registry or is otherwise invalid.
    pub fn operator(&self, id: OperatorId) -> Result<&BinaryOperatorDescriptor, CoreError> {
        if id.registry_id != self.registry_id {
            return Err(CoreError::UnknownOperatorId(id));
        }
        self.operators
            .get(id.index)
            .ok_or(CoreError::UnknownOperatorId(id))
    }
}
