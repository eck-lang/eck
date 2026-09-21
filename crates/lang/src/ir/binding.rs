use crate::semantic::SemanticType;
use crate::syntax::Span;

/// Identifies one semantic binding independently from its runtime storage slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindingId(pub usize);

/// The assignment promise created by one source declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingContract {
    /// Every assigned value must satisfy the explicit or immutable type.
    Static(SemanticType),
    /// A mutable unannotated binding accepts every concrete ECK value.
    Dynamic,
}

/// Records the declaration information needed by later static-analysis passes.
#[derive(Clone)]
pub struct BindingMetadata {
    pub id: BindingId,
    pub slot: LocalVariableSlot,
    pub name: String,
    pub mutable: bool,
    pub contract: BindingContract,
    pub semantic_type: SemanticType,
    pub declaration_span: Span,
    pub scope_depth: usize,
}

/// Identifies one statically allocated local value slot in a typed program.
///
/// The compiler assigns each declaration a distinct slot. Runtime execution
/// uses the slot directly instead of resolving the source name repeatedly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalVariableSlot(pub usize);
