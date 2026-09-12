use crate::binding::BindingMetadata;
use crate::statement::TypedStatement;

/// A compiled program that is ready for execution.
///
/// The compiler resolves every name, type, operator, comparison, and function
/// before producing this value, so the runtime executes the statement tree
/// without repeating any of that work.
#[derive(Clone)]
pub struct TypedProgram {
    pub statements: Vec<TypedStatement>,
    pub local_slot_count: usize,
    pub bindings: Vec<BindingMetadata>,
}
