//! Subtype registration, qualified arithmetic, and direct conversions.

use std::collections::HashSet;

use crate::{
    BinaryOperator, CoreError, ResolvedBinaryOperator, ResolvedSubtypeConversion, Scale,
    SubtypeBinaryRule, SubtypeDescriptor, SubtypeId, ValueType,
};

use super::Registry;

impl Registry {
    /// Registers a semantic subtype and every literal suffix that names it.
    ///
    /// Subtype names and suffixes are globally unique. The descriptor ID must
    /// have been allocated by this registry and may be registered exactly once.
    /// The first suffix is the canonical form later used when formatting
    /// qualified values.
    pub fn register_subtype(&mut self, descriptor: SubtypeDescriptor) -> Result<(), CoreError> {
        let id = descriptor.id;
        if self.subtypes_by_name.contains_key(descriptor.name) {
            return Err(CoreError::DuplicateSubtype(descriptor.name.to_string()));
        }
        if self.subtypes.contains_key(&id) {
            return Err(CoreError::DuplicateSubtypeId(id));
        }
        if !self.allocated_subtype_ids.contains(&id) {
            return Err(CoreError::UnallocatedSubtypeId(id));
        }
        if descriptor.suffixes.is_empty() {
            return Err(CoreError::MissingLiteralSuffix(descriptor.name.to_string()));
        }

        let mut unique_suffixes = HashSet::with_capacity(descriptor.suffixes.len());
        for suffix in descriptor.suffixes {
            if !unique_suffixes.insert(*suffix) || self.subtypes_by_suffix.contains_key(suffix) {
                return Err(CoreError::DuplicateLiteralSuffix((*suffix).to_string()));
            }
        }

        self.subtypes_by_name.insert(descriptor.name, id);
        for suffix in descriptor.suffixes {
            self.subtypes_by_suffix.insert(*suffix, id);
        }
        self.subtypes.insert(id, descriptor);
        self.allocated_subtype_ids.remove(&id);
        Ok(())
    }

    /// Resolves a semantic subtype name to its registered ID.
    pub fn subtype_by_name(&self, name: &str) -> Option<SubtypeId> {
        self.subtypes_by_name.get(name).copied()
    }

    /// Resolves a literal suffix such as `mm` to its registered subtype ID.
    pub fn subtype_by_suffix(&self, suffix: &str) -> Option<SubtypeId> {
        self.subtypes_by_suffix.get(suffix).copied()
    }

    /// Returns a snapshot iterator over every registered subtype ID.
    ///
    /// IDs are yielded in their registry allocation order, rather than the
    /// unspecified iteration order of the registry's hash-map index. The
    /// iterator includes only fully registered subtypes; IDs that have been
    /// allocated but not registered are omitted. This allows an extension to
    /// register rules against capabilities installed by earlier extensions
    /// without depending on their concrete crates.
    pub fn registered_subtype_ids(&self) -> impl Iterator<Item = SubtypeId> + use<> {
        let mut subtype_ids = self.subtypes.keys().copied().collect::<Vec<_>>();
        subtype_ids.sort_unstable_by_key(|id| id.index);
        subtype_ids.into_iter()
    }

    /// Returns the descriptor for a registered subtype ID.
    pub fn subtype_descriptor(&self, id: SubtypeId) -> Result<&SubtypeDescriptor, CoreError> {
        self.subtypes
            .get(&id)
            .ok_or(CoreError::UnknownSubtypeId(id))
    }

    /// Registers how one binary operation combines two optional subtypes.
    ///
    /// `None` denotes a plain, unqualified value. The rule can choose an output
    /// subtype and independent scale factors for both input magnitudes.
    /// A rule must qualify at least one operand: the registry resolves two plain
    /// operands directly through their base-type operator and never consults a
    /// subtype rule for them.
    ///
    /// Every present input or output subtype must already be registered. Returns
    /// [`CoreError::UnknownSubtypeId`] for an unknown subtype,
    /// [`CoreError::UnreachableSubtypeOperatorRule`] when both inputs are plain,
    /// [`CoreError::InvalidScale`] for a zero denominator, or
    /// [`CoreError::DuplicateSubtypeOperator`] for an existing rule.
    pub fn register_subtype_binary_rule(
        &mut self,
        operator: BinaryOperator,
        left_operand_subtype: Option<SubtypeId>,
        right_operand_subtype: Option<SubtypeId>,
        rule: SubtypeBinaryRule,
    ) -> Result<(), CoreError> {
        if left_operand_subtype.is_none() && right_operand_subtype.is_none() {
            return Err(CoreError::UnreachableSubtypeOperatorRule(operator));
        }
        self.validate_optional_subtype(left_operand_subtype)?;
        self.validate_optional_subtype(right_operand_subtype)?;
        self.validate_optional_subtype(rule.output)?;
        if rule.left_operand_scale.denominator == 0 || rule.right_operand_scale.denominator == 0 {
            return Err(CoreError::InvalidScale);
        }

        let key = (operator, left_operand_subtype, right_operand_subtype);
        if self.subtype_operator_index.contains_key(&key) {
            return Err(CoreError::DuplicateSubtypeOperator {
                operator,
                left_operand_subtype: self.optional_subtype_name(left_operand_subtype),
                right_operand_subtype: self.optional_subtype_name(right_operand_subtype),
            });
        }
        self.subtype_operator_index.insert(key, rule);
        Ok(())
    }

    /// Resolves a binary operation for fully qualified operand types.
    ///
    /// Plain operands resolve directly to their exact base-type operator. For
    /// qualified operands, a subtype rule first determines any magnitude
    /// scales; the executable operation is then selected for the resulting
    /// base types. No fallback or implicit subtype conversion is attempted.
    pub fn resolve_binary_operation(
        &self,
        operator: BinaryOperator,
        left_operand_type: ValueType,
        right_operand_type: ValueType,
    ) -> Result<ResolvedBinaryOperator, CoreError> {
        if left_operand_type.subtype.is_none() && right_operand_type.subtype.is_none() {
            let resolved_operator = self.resolve_binary_operator(
                operator,
                left_operand_type.base,
                right_operand_type.base,
            )?;
            let result_base_type = self.operator(resolved_operator)?.result_type;
            return Ok(ResolvedBinaryOperator {
                operator: resolved_operator,
                output: ValueType::plain(result_base_type),
                left_operand_scale: Scale::IDENTITY,
                right_operand_scale: Scale::IDENTITY,
            });
        }

        let rule = self
            .subtype_operator_index
            .get(&(
                operator,
                left_operand_type.subtype,
                right_operand_type.subtype,
            ))
            .copied()
            .ok_or_else(|| CoreError::SubtypeOperatorNotDefined {
                operator,
                left_operand_type: self.value_type_name(left_operand_type),
                right_operand_type: self.value_type_name(right_operand_type),
            })?;
        let scaled_left_operand_type =
            self.resolve_scaled_base_type(left_operand_type.base, rule.left_operand_scale)?;
        let scaled_right_operand_type =
            self.resolve_scaled_base_type(right_operand_type.base, rule.right_operand_scale)?;
        let resolved_operator = self.resolve_binary_operator(
            operator,
            scaled_left_operand_type,
            scaled_right_operand_type,
        )?;
        let result_base_type = self.operator(resolved_operator)?.result_type;

        Ok(ResolvedBinaryOperator {
            operator: resolved_operator,
            output: ValueType {
                base: result_base_type,
                subtype: rule.output,
            },
            left_operand_scale: rule.left_operand_scale,
            right_operand_scale: rule.right_operand_scale,
        })
    }

    /// Registers a direct scale conversion from one subtype to another.
    ///
    /// Extensions must register each direction explicitly when both directions
    /// are valid, allowing the registry to remain deterministic and avoid graph
    /// traversal on the conversion hot path. A same-subtype conversion must use
    /// the identity scale because it is resolved without an index lookup. Both
    /// subtype IDs must already be registered. Returns
    /// [`CoreError::UnknownSubtypeId`] for an unknown ID,
    /// [`CoreError::InvalidScale`] for a zero denominator,
    /// [`CoreError::InvalidIdentitySubtypeConversion`] for a non-identity self
    /// conversion, or [`CoreError::DuplicateSubtypeConversion`] for an existing
    /// conversion.
    pub fn register_subtype_conversion(
        &mut self,
        from: SubtypeId,
        to: SubtypeId,
        scale: Scale,
    ) -> Result<(), CoreError> {
        self.subtype_descriptor(from)?;
        self.subtype_descriptor(to)?;
        if scale.denominator == 0 {
            return Err(CoreError::InvalidScale);
        }
        if from == to && !scale.is_identity() {
            return Err(CoreError::InvalidIdentitySubtypeConversion(
                self.optional_subtype_name(Some(from)),
            ));
        }
        if self.subtype_conversion_index.contains_key(&(from, to)) {
            return Err(CoreError::DuplicateSubtypeConversion {
                from: self.optional_subtype_name(Some(from)),
                to: self.optional_subtype_name(Some(to)),
            });
        }
        self.subtype_conversion_index.insert((from, to), scale);
        Ok(())
    }

    /// Resolves a direct subtype conversion and its resulting qualified type.
    ///
    /// A conversion may promote an integer base value to the configured default
    /// fractional type when its scale requires division, preserving fractional
    /// information rather than truncating it. Returns [`CoreError::UnknownTypeId`]
    /// or [`CoreError::UnknownSubtypeId`] for invalid input references, and
    /// [`CoreError::SubtypeConversionNotDefined`] when no direct conversion exists.
    pub fn resolve_subtype_conversion(
        &self,
        source: ValueType,
        target: SubtypeId,
    ) -> Result<ResolvedSubtypeConversion, CoreError> {
        self.type_descriptor(source.base)?;
        let Some(source_subtype) = source.subtype else {
            return Err(CoreError::SubtypeConversionNotDefined {
                from: self.value_type_name(source),
                to: self.optional_subtype_name(Some(target)),
            });
        };
        self.subtype_descriptor(source_subtype)?;
        self.subtype_descriptor(target)?;
        let scale = if source_subtype == target {
            Scale::IDENTITY
        } else {
            self.subtype_conversion_index
                .get(&(source_subtype, target))
                .copied()
                .ok_or_else(|| CoreError::SubtypeConversionNotDefined {
                    from: self.optional_subtype_name(Some(source_subtype)),
                    to: self.optional_subtype_name(Some(target)),
                })?
        };

        let output_base = self.resolve_scaled_base_type(source.base, scale)?;

        Ok(ResolvedSubtypeConversion {
            output: ValueType::qualified(output_base, target),
            scale,
        })
    }

    /// Resolves the base type produced by applying a magnitude scale.
    ///
    /// Scaling follows the same operator sequence used by the runtime: it
    /// first multiplies by the numerator using the current type, then divides
    /// by the denominator. Division of the configured default integer type
    /// uses the configured default fractional type as its divisor so a
    /// fractional scale does not truncate its magnitude.
    pub(super) fn resolve_scaled_base_type(
        &self,
        base_type: crate::TypeId,
        scale: Scale,
    ) -> Result<crate::TypeId, CoreError> {
        let mut scaled_base_type = base_type;
        if scale.numerator != 1 {
            let operator = self.resolve_binary_operator(
                BinaryOperator::Multiplication,
                scaled_base_type,
                scaled_base_type,
            )?;
            scaled_base_type = self.operator(operator)?.result_type;
        }
        if scale.denominator != 1 {
            let divisor_type = if self.default_integer == Some(scaled_base_type) {
                self.default_fractional()?
            } else {
                scaled_base_type
            };
            let operator = self.resolve_binary_operator(
                BinaryOperator::Division,
                scaled_base_type,
                divisor_type,
            )?;
            scaled_base_type = self.operator(operator)?.result_type;
        }
        Ok(scaled_base_type)
    }

    /// Returns a subtype name for diagnostics, including a marker for plain values.
    pub(super) fn optional_subtype_name(&self, id: Option<SubtypeId>) -> String {
        match id {
            Some(id) => self
                .subtypes
                .get(&id)
                .map(|subtype| subtype.name.to_string())
                .unwrap_or_else(|| "<unknown>".to_string()),
            None => "<plain>".to_string(),
        }
    }

    /// Verifies that an optional subtype reference is either plain or registered.
    pub(super) fn validate_optional_subtype(&self, id: Option<SubtypeId>) -> Result<(), CoreError> {
        if let Some(id) = id {
            self.subtype_descriptor(id)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "subtypes.tests.rs"]
mod tests;
