//! Base-type comparison registration and subtype-aware resolution.

use crate::{
    ComparisonDescriptor, ComparisonExecutor, ComparisonId, ComparisonOperator, CoreError,
    ResolvedComparison, SubtypeComparisonRule, SubtypeId, TypeId, ValueType,
};

use super::Registry;

impl Registry {
    /// Registers one comparison implementation for an exact pair of base types.
    ///
    /// Both type IDs must already be registered. The comparison result is
    /// always resolved to the configured default boolean type, so registering
    /// a base comparator does not require a boolean type to exist yet. Returns
    /// [`CoreError::UnknownTypeId`] for an unknown type and
    /// [`CoreError::DuplicateComparison`] for an existing signature.
    pub fn register_comparison(
        &mut self,
        operator: ComparisonOperator,
        left_operand_type: TypeId,
        right_operand_type: TypeId,
        execute: ComparisonExecutor,
    ) -> Result<ComparisonId, CoreError> {
        self.type_descriptor(left_operand_type)?;
        self.type_descriptor(right_operand_type)?;
        let key = (operator, left_operand_type, right_operand_type);
        if self.comparison_index.contains_key(&key) {
            return Err(CoreError::DuplicateComparison {
                operator,
                left_operand_type: self.type_name(left_operand_type).to_string(),
                right_operand_type: self.type_name(right_operand_type).to_string(),
            });
        }

        let id = ComparisonId {
            registry_id: self.registry_id,
            index: self.comparisons.len(),
        };
        self.comparisons.push(ComparisonDescriptor {
            id,
            operator,
            left_operand_type,
            right_operand_type,
            execute,
        });
        self.comparison_index.insert(key, id);
        Ok(id)
    }

    /// Resolves a comparison for an exact pair of base types.
    ///
    /// This method does not apply subtype rules or implicit coercions. Returns
    /// [`CoreError::ComparisonNotDefined`] if no exact comparison is installed.
    pub fn resolve_comparison(
        &self,
        operator: ComparisonOperator,
        left_operand_type: TypeId,
        right_operand_type: TypeId,
    ) -> Result<ComparisonId, CoreError> {
        self.comparison_index
            .get(&(operator, left_operand_type, right_operand_type))
            .copied()
            .ok_or_else(|| CoreError::ComparisonNotDefined {
                operator,
                left_operand_type: self.type_name(left_operand_type).to_string(),
                right_operand_type: self.type_name(right_operand_type).to_string(),
            })
    }

    /// Returns the executable descriptor identified by a resolved comparison ID.
    ///
    /// Returns [`CoreError::UnknownComparisonId`] when the ID belongs to a
    /// different registry or does not identify a registered descriptor.
    pub fn comparison(&self, id: ComparisonId) -> Result<&ComparisonDescriptor, CoreError> {
        if id.registry_id != self.registry_id {
            return Err(CoreError::UnknownComparisonId(id));
        }
        self.comparisons
            .get(id.index)
            .ok_or(CoreError::UnknownComparisonId(id))
    }

    /// Registers how one comparison accepts a pair of optional subtypes.
    ///
    /// `None` represents a plain operand. At least one operand must be
    /// qualified because two plain operands resolve straight to a base
    /// comparison. Rules provide only operand scales; comparisons always
    /// return a plain boolean. Returns validation errors for unknown subtypes,
    /// zero scale denominators, duplicate rules, or two plain operands.
    pub fn register_subtype_comparison_rule(
        &mut self,
        operator: ComparisonOperator,
        left_operand_subtype: Option<SubtypeId>,
        right_operand_subtype: Option<SubtypeId>,
        rule: SubtypeComparisonRule,
    ) -> Result<(), CoreError> {
        if left_operand_subtype.is_none() && right_operand_subtype.is_none() {
            return Err(CoreError::UnreachableSubtypeComparisonRule(operator));
        }
        self.validate_optional_subtype(left_operand_subtype)?;
        self.validate_optional_subtype(right_operand_subtype)?;
        if rule.left_operand_scale.denominator == 0 || rule.right_operand_scale.denominator == 0 {
            return Err(CoreError::InvalidScale);
        }
        let key = (operator, left_operand_subtype, right_operand_subtype);
        if self.subtype_comparison_index.contains_key(&key) {
            return Err(CoreError::DuplicateSubtypeComparison {
                operator,
                left_operand_subtype: self.optional_subtype_name(left_operand_subtype),
                right_operand_subtype: self.optional_subtype_name(right_operand_subtype),
            });
        }
        self.subtype_comparison_index.insert(key, rule);
        Ok(())
    }

    /// Resolves a comparison for fully qualified operand types.
    ///
    /// Qualified operands require an exact subtype rule. The rule's scales may
    /// promote their base types before the executable comparison is selected.
    /// The output is always a plain value of [`Registry::default_boolean`].
    pub fn resolve_comparison_operation(
        &self,
        operator: ComparisonOperator,
        left_operand_type: ValueType,
        right_operand_type: ValueType,
    ) -> Result<ResolvedComparison, CoreError> {
        let (left_operand_scale, right_operand_scale) =
            if left_operand_type.subtype.is_none() && right_operand_type.subtype.is_none() {
                (crate::Scale::IDENTITY, crate::Scale::IDENTITY)
            } else {
                let rule = self
                    .subtype_comparison_index
                    .get(&(
                        operator,
                        left_operand_type.subtype,
                        right_operand_type.subtype,
                    ))
                    .copied()
                    .ok_or_else(|| CoreError::SubtypeComparisonNotDefined {
                        operator,
                        left_operand_type: self.value_type_name(left_operand_type),
                        right_operand_type: self.value_type_name(right_operand_type),
                    })?;
                (rule.left_operand_scale, rule.right_operand_scale)
            };
        let scaled_left_operand_type =
            self.resolve_scaled_base_type(left_operand_type.base, left_operand_scale)?;
        let scaled_right_operand_type =
            self.resolve_scaled_base_type(right_operand_type.base, right_operand_scale)?;
        let comparison = self.resolve_comparison(
            operator,
            scaled_left_operand_type,
            scaled_right_operand_type,
        )?;
        Ok(ResolvedComparison {
            comparison,
            output: ValueType::plain(self.default_boolean()?),
            left_operand_scale,
            right_operand_scale,
        })
    }
}

#[cfg(test)]
#[path = "comparisons.tests.rs"]
mod tests;
