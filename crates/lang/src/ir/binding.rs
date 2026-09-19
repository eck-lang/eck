use crate::semantic::SemanticType;
use crate::syntax::Span;

/// Identifies one semantic binding independently from its runtime storage slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindingId(pub usize);

/// Records the declaration information needed by later static-analysis passes.
#[derive(Clone)]
pub struct BindingMetadata {
    pub id: BindingId,
    pub slot: LocalVariableSlot,
    pub name: String,
    pub mutable: bool,
    pub semantic_type: SemanticType,
    pub nullable: bool,
    pub declaration_span: Span,
    pub scope_depth: usize,
}

/// Identifies one statically allocated local value slot in a typed program.
///
/// The compiler assigns each declaration a distinct slot. Runtime execution
/// uses the slot directly instead of resolving the source name repeatedly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalVariableSlot(pub usize);
