//! Reusable fixtures for registry unit tests.
//!
//! The helpers in this module construct only the minimal valid inputs needed
//! to exercise registry validation and resolution. Their callback bodies are
//! deliberately simple: registry tests verify descriptor registration and
//! lookup, not the concrete execution behaviour owned by an extension.

use crate::{
    CoreError, ExecutionContext, Registry, SubtypeDescriptor, SubtypeId, TypeDescriptor, TypeId,
    Value,
};

/// A no-op formatter for descriptors that only need to satisfy registration.
///
/// Returning an empty string prevents tests from depending on a particular
/// value representation while still providing the formatter required by every
/// [`TypeDescriptor`].
pub(super) fn format_value(_: &Value) -> Result<String, CoreError> {
    Ok(String::new())
}

/// Reads the Rust boolean payload used by registry fixtures.
///
/// Tests that configure a default boolean use this evaluator to satisfy the
/// extension-owned representation contract without depending on `eck-primitives`.
pub(super) fn evaluate_boolean(value: &Value) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("test bool".into()))
}

/// An operator callback that returns its left operand unchanged.
///
/// Tests use this as a valid executable callback when their subject is
/// operator registration or resolution. The identity result is intentional:
/// no test using this helper should infer arithmetic semantics from it.
pub(super) fn execute_operator(left_operand: &Value, _: &Value) -> Result<Value, CoreError> {
    Ok(left_operand.clone())
}

/// A function callback that reports no value.
///
/// This supplies the required execution function for descriptor tests without
/// imposing result semantics; function execution is outside the scope of the
/// registry tests that use it.
pub(super) fn execute_function(
    _: &ExecutionContext<'_>,
    _: &[Value],
) -> Result<Option<Value>, CoreError> {
    Ok(None)
}

/// Builds a minimal valid descriptor for a type ID allocated by a registry.
///
/// The descriptor intentionally supports no literal parsing because tests that
/// use it exercise registration and lookup only. Its formatter is the no-op
/// [`format_value`] fixture above.
pub(super) fn type_descriptor(id: TypeId, name: &'static str) -> TypeDescriptor {
    TypeDescriptor {
        id,
        name,
        parse_numeric_literal: None,
        parse_string_literal: None,
        parse_boolean_literal: None,
        format: format_value,
    }
}

/// Allocates and registers a minimal base type, returning its registry-local ID.
///
/// This combines the setup steps required by most registry tests. It unwraps
/// registration because callers must provide a fresh name and an otherwise
/// valid registry; failure cases are tested explicitly at their call sites.
pub(super) fn register_type(registry: &mut Registry, name: &'static str) -> TypeId {
    let id = registry.allocate_type_id();
    registry.register_type(type_descriptor(id, name)).unwrap();
    id
}

/// Allocates and registers a minimal subtype with the shared `unit` suffix.
///
/// The constant suffix is sufficient for tests that only need a registered
/// subtype. Callers testing suffix conflicts or exact suffix lookup should
/// construct their own [`SubtypeDescriptor`] instead.
pub(super) fn register_subtype(registry: &mut Registry, name: &'static str) -> SubtypeId {
    let id = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id,
            name,
            suffixes: &["unit"],
        })
        .unwrap();
    id
}

/// Produces a type ID allocated by a different registry instance.
///
/// Registry IDs are instance-scoped, so this value lets a test distinguish an
/// ID that is structurally allocated from one that belongs to the registry
/// under test. It is useful for validating ownership and unknown-ID errors.
pub(super) fn foreign_type_id() -> TypeId {
    Registry::new().allocate_type_id()
}

/// Produces a subtype ID allocated by a different registry instance.
///
/// Like [`foreign_type_id`], the returned ID is valid only for its temporary
/// source registry and is intended for assertions about registry-local ID
/// validation.
pub(super) fn foreign_subtype_id() -> SubtypeId {
    Registry::new().allocate_subtype_id()
}
