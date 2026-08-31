//! Type registration, literal parsing, defaults, and value formatting.

use crate::{BooleanEvaluator, CoreError, TypeDescriptor, TypeId, Value, ValueType};

use super::Registry;

impl Registry {
    /// Registers a concrete base type and the literal/formatting hooks it owns.
    ///
    /// Type names are globally unique within a registry. The descriptor ID must
    /// have been allocated by this registry and may be registered exactly once.
    pub fn register_type(&mut self, descriptor: TypeDescriptor) -> Result<(), CoreError> {
        let id = descriptor.id;
        if self.types_by_name.contains_key(descriptor.name) {
            return Err(CoreError::DuplicateType(descriptor.name.to_string()));
        }
        if self.types.contains_key(&id) {
            return Err(CoreError::DuplicateTypeId(id));
        }
        if !self.allocated_type_ids.contains(&id) {
            return Err(CoreError::UnallocatedTypeId(id));
        }

        let name = descriptor.name;
        self.types_by_name.insert(name, id);
        self.types.insert(id, descriptor);
        self.allocated_type_ids.remove(&id);
        self.activate_comparison_declarations_for(name);
        Ok(())
    }

    /// Resolves a declared type name to its registered ID.
    ///
    /// Returns `None` when no extension registered that exact name.
    pub fn type_by_name(&self, name: &str) -> Option<TypeId> {
        self.types_by_name.get(name).copied()
    }

    /// Returns the descriptor for a registered base type ID.
    ///
    /// The error preserves the unresolved ID so callers can report an internal
    /// registration or resolution failure precisely.
    pub fn type_descriptor(&self, id: TypeId) -> Result<&TypeDescriptor, CoreError> {
        self.types.get(&id).ok_or(CoreError::UnknownTypeId(id))
    }

    /// Returns a registered type name, or a stable placeholder for an unknown ID.
    ///
    /// This method is intended for diagnostics, where producing a useful error
    /// must not hide the original failure behind another lookup error.
    pub fn type_name(&self, id: TypeId) -> &str {
        self.types
            .get(&id)
            .map(|type_descriptor| type_descriptor.name)
            .unwrap_or("<unknown>")
    }

    /// Configures the type selected for integer literals without explicit context.
    ///
    /// Returns [`CoreError::UnknownTypeId`] when `id` is not registered, leaving
    /// the previously configured default unchanged.
    pub fn set_default_integer(&mut self, id: TypeId) -> Result<(), CoreError> {
        self.type_descriptor(id)?;
        self.default_integer = Some(id);
        Ok(())
    }

    /// Configures the type selected for fractional literals without explicit context.
    ///
    /// Returns [`CoreError::UnknownTypeId`] when `id` is not registered, leaving
    /// the previously configured default unchanged.
    pub fn set_default_fractional(&mut self, id: TypeId) -> Result<(), CoreError> {
        self.type_descriptor(id)?;
        self.default_fractional = Some(id);
        Ok(())
    }

    /// Configures the type selected for string literals without explicit context.
    ///
    /// Returns [`CoreError::UnknownTypeId`] when `id` is not registered, leaving
    /// the previously configured default unchanged.
    pub fn set_default_string(&mut self, id: TypeId) -> Result<(), CoreError> {
        self.type_descriptor(id)?;
        self.default_string = Some(id);
        Ok(())
    }

    /// Returns the configured default integer type.
    pub fn default_integer(&self) -> Result<TypeId, CoreError> {
        self.default_integer
            .ok_or(CoreError::MissingDefault("integer"))
    }

    /// Returns the configured default fractional type.
    pub fn default_fractional(&self) -> Result<TypeId, CoreError> {
        self.default_fractional
            .ok_or(CoreError::MissingDefault("fractional"))
    }

    /// Returns the configured default string type.
    pub fn default_string(&self) -> Result<TypeId, CoreError> {
        self.default_string
            .ok_or(CoreError::MissingDefault("string"))
    }

    /// Configures the type used by boolean literals and conditional evaluation.
    ///
    /// `evaluate` belongs to the type extension and validates its opaque value
    /// representation before returning the language-level truth value. Returns
    /// [`CoreError::UnknownTypeId`] when `id` is not registered, leaving the
    /// previously configured default unchanged.
    pub fn set_default_boolean(
        &mut self,
        id: TypeId,
        evaluate: BooleanEvaluator,
    ) -> Result<(), CoreError> {
        self.type_descriptor(id)?;
        self.default_boolean = Some((id, evaluate));
        Ok(())
    }

    /// Returns the configured default boolean type.
    pub fn default_boolean(&self) -> Result<TypeId, CoreError> {
        self.default_boolean
            .map(|(id, _)| id)
            .ok_or(CoreError::MissingDefault("boolean"))
    }

    /// Configures the type used by null literals.
    ///
    /// Returns [`CoreError::UnknownTypeId`] when `id` is not registered, leaving
    /// the previously configured default unchanged.
    pub fn set_default_null(&mut self, id: TypeId) -> Result<(), CoreError> {
        self.type_descriptor(id)?;
        self.default_null = Some(id);
        Ok(())
    }

    /// Returns the configured default null type.
    pub fn default_null(&self) -> Result<TypeId, CoreError> {
        self.default_null.ok_or(CoreError::MissingDefault("null"))
    }

    /// Evaluates a plain value of the configured default boolean type.
    ///
    /// The registered type extension owns validation of the opaque payload.
    /// Values with another base type or a subtype qualifier are rejected before
    /// invoking that evaluator.
    pub fn evaluate_boolean(&self, value: &Value) -> Result<bool, CoreError> {
        let (boolean_type, evaluate) = self
            .default_boolean
            .ok_or(CoreError::MissingDefault("boolean"))?;
        let expected = ValueType::plain(boolean_type);
        let actual = value.value_type();
        if actual != expected {
            return Err(CoreError::UnexpectedBooleanValueType {
                expected: self.value_type_name(expected),
                actual: self.value_type_name(actual),
            });
        }
        evaluate(value)
    }

    /// Parses a numeric token through its expected or inferred registered type.
    ///
    /// Explicit context always wins. Without it, an integer-shaped token uses
    /// the default integer type and a token containing `.` or an exponent marker
    /// uses the default fractional type.
    pub fn parse_numeric(
        &self,
        raw_text: &str,
        expected: Option<TypeId>,
    ) -> Result<Value, CoreError> {
        let type_id = match expected {
            Some(id) => id,
            None if raw_text.contains(['.', 'e', 'E']) => self.default_fractional()?,
            None => self.default_integer()?,
        };
        let type_descriptor = self.type_descriptor(type_id)?;
        let parser =
            type_descriptor
                .parse_numeric_literal
                .ok_or_else(|| CoreError::UnsupportedLiteral {
                    type_name: type_descriptor.name.to_string(),
                    literal_kind: "numeric",
                })?;
        parser(raw_text, type_id)
    }

    /// Parses a string token through its expected or default registered type.
    pub fn parse_string(
        &self,
        raw_text: &str,
        expected: Option<TypeId>,
    ) -> Result<Value, CoreError> {
        let type_id = expected.unwrap_or(self.default_string()?);
        let type_descriptor = self.type_descriptor(type_id)?;
        let parser =
            type_descriptor
                .parse_string_literal
                .ok_or_else(|| CoreError::UnsupportedLiteral {
                    type_name: type_descriptor.name.to_string(),
                    literal_kind: "string",
                })?;
        parser(raw_text, type_id)
    }

    /// Parses a boolean token through its expected or default registered type.
    ///
    /// Explicit context takes precedence. Without it, the configured default
    /// boolean type is used. Returns [`CoreError::UnsupportedLiteral`] when
    /// the selected type does not accept boolean literals.
    pub fn parse_boolean(
        &self,
        raw_text: &str,
        expected: Option<TypeId>,
    ) -> Result<Value, CoreError> {
        let type_id = expected.unwrap_or(self.default_boolean()?);
        let type_descriptor = self.type_descriptor(type_id)?;
        let parser =
            type_descriptor
                .parse_boolean_literal
                .ok_or_else(|| CoreError::UnsupportedLiteral {
                    type_name: type_descriptor.name.to_string(),
                    literal_kind: "boolean",
                })?;
        parser(raw_text, type_id)
    }

    /// Parses a null token through its expected or default registered type.
    ///
    /// Explicit context takes precedence. Without it, the configured default
    /// null type is used. Returns [`CoreError::UnsupportedLiteral`] when
    /// the selected type does not accept null literals.
    pub fn parse_null(
        &self,
        raw_text: &str,
        expected: Option<TypeId>,
    ) -> Result<Value, CoreError> {
        let type_id = expected.unwrap_or(self.default_null()?);
        let type_descriptor = self.type_descriptor(type_id)?;
        let parser =
            type_descriptor
                .parse_null_literal
                .ok_or_else(|| CoreError::UnsupportedLiteral {
                    type_name: type_descriptor.name.to_string(),
                    literal_kind: "null",
                })?;
        parser(raw_text, type_id)
    }

    /// Formats a value with its type formatter and canonical subtype suffix.
    ///
    /// The formatter receives only the underlying value. The registry appends
    /// the canonical suffix afterwards so every formatter treats qualified and
    /// plain values consistently.
    pub fn format_value(&self, value: &Value) -> Result<String, CoreError> {
        self.format_value_with_configuration(value, &self.default_runtime_configuration())
    }

    /// Formats a base type plus an optional subtype for diagnostics.
    pub fn value_type_name(&self, value_type: ValueType) -> String {
        match value_type.subtype {
            Some(id) => match self.subtypes.get(&id) {
                Some(subtype) => format!(
                    "{}[{}]",
                    self.type_name(value_type.base),
                    subtype.canonical_suffix()
                ),
                None => format!("{}[<unknown>]", self.type_name(value_type.base)),
            },
            None => self.type_name(value_type.base).to_string(),
        }
    }
}

#[cfg(test)]
#[path = "types.tests.rs"]
mod tests;
