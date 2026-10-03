use crate::ir::binding::BindingMetadata;
use crate::ir::statement::TypedStatement;

/// A compiled program that is ready for execution.
///
/// The compiler resolves every name, type, operator, comparison, and function
/// before producing this value, so the runtime executes the statement tree
/// without repeating any of that work.
#[derive(Clone)]
pub struct TypedProgram {
    /// Registry callback identity against which parallel safety was proven.
    pub(crate) execution_identity: Option<(u64, u64)>,
    /// Compile-time dependency and effect decisions consumed by execution.
    pub(crate) execution_analysis: crate::analysis::ExecutionAnalysis,
    pub(crate) statements: Vec<TypedStatement>,
    pub(crate) local_slot_count: usize,
    pub(crate) bindings: Vec<BindingMetadata>,
}

impl TypedProgram {
    /// Constructs manually resolved IR with the sequential execution fallback.
    ///
    /// Automatic parallel execution requires a proof from the compiler. Raw IR
    /// construction intentionally cannot supply or forge that proof.
    pub fn from_parts(
        statements: Vec<TypedStatement>,
        local_slot_count: usize,
        bindings: Vec<BindingMetadata>,
    ) -> Self {
        Self {
            statements,
            local_slot_count,
            bindings,
            execution_analysis: Default::default(),
            execution_identity: None,
        }
    }

    /// Borrows the immutable resolved statement tree analyzed by the compiler.
    pub fn statements(&self) -> &[TypedStatement] {
        &self.statements
    }

    /// Invalidates execution proofs before allowing a caller to rewrite the IR.
    ///
    /// An edited program executes sequentially until it is compiled again. A
    /// source span alone is never sufficient evidence for a replacement body.
    pub fn statements_mut(&mut self) -> &mut Vec<TypedStatement> {
        self.execution_analysis = Default::default();
        self.execution_identity = None;
        &mut self.statements
    }

    /// Returns the number of slots reserved for one invocation's locals.
    pub fn local_slot_count(&self) -> usize {
        self.local_slot_count
    }

    /// Borrows the resolved declaration identities and contracts.
    pub fn bindings(&self) -> &[BindingMetadata] {
        &self.bindings
    }

    /// Borrows structured execution effects, dependencies and loop decisions.
    pub fn execution_analysis(&self) -> &crate::analysis::ExecutionAnalysis {
        &self.execution_analysis
    }
}
