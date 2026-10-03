//! Structured resources, dependencies, and reasons retained by execution analysis.

use crate::ir::{BindingId, LocalVariableSlot};
use crate::semantic::FunctionId;
use crate::syntax::Span;

/// One resolved storage identity; textual names never participate in dependence proofs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindingResource {
    pub binding: BindingId,
    pub slot: LocalVariableSlot,
}

/// Exact integer index `coefficient * iteration + offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AffineIndex {
    pub iteration: BindingResource,
    pub coefficient: i128,
    pub offset: i128,
}

/// A region of resolved local storage accessed by an expression or statement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Resource {
    Binding(BindingResource),
    ArrayElement {
        container: BindingResource,
        index: IndexAccess,
    },
    Map {
        slot: LocalVariableSlot,
    },
}

impl Resource {
    /// Returns the runtime slot containing this resource.
    pub fn slot(&self) -> LocalVariableSlot {
        match self {
            Self::Binding(binding) => binding.slot,
            Self::ArrayElement { container, .. } => container.slot,
            Self::Map { slot } => *slot,
        }
    }
}

/// An index proof; unknown accesses cannot establish disjointness.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexAccess {
    Constant(i128),
    Affine(AffineIndex),
    Unknown,
}

/// One access and its source location, retained for dependency explanations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceAccess {
    pub resource: Resource,
    pub span: Span,
}

/// A category of state outside worker-private local storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalEffect {
    Read,
    Write,
    Unknown,
    Configuration,
}

/// Why a region cannot be evaluated as independent iterations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SequentialReason {
    CapturedMutation(BindingResource),
    ConflictingAccesses,
    NonAffineWrite(BindingResource),
    WholeContainerRead(BindingResource),
    LocalArrayMutation(BindingResource),
    UnsupportedMutation,
    MapOperation,
    UnsafeCall(FunctionId),
    UnknownOperation,
    UnsupportedConstruct,
    UnsupportedRange,
    NestedLoop,
    Break,
    Configuration,
    DuplicateLoopSpan,
}

/// Effects compose by union, determinism conjunction, and failure disjunction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectSummary {
    pub reads: Vec<ResourceAccess>,
    pub writes: Vec<ResourceAccess>,
    pub external_effects: Vec<ExternalEffect>,
    pub deterministic: bool,
    pub may_fail: bool,
    pub reasons: Vec<SequentialReason>,
}

impl Default for EffectSummary {
    /// Creates the identity summary for an empty computation.
    fn default() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
            external_effects: Vec::new(),
            deterministic: true,
            may_fail: false,
            reasons: Vec::new(),
        }
    }
}

impl EffectSummary {
    /// Combines execution regions without dropping alternate branch effects.
    pub fn combine(&mut self, other: &Self) {
        self.reads.extend(other.reads.iter().cloned());
        self.writes.extend(other.writes.iter().cloned());
        for effect in &other.external_effects {
            if !self.external_effects.contains(effect) {
                self.external_effects.push(effect.clone());
            }
        }
        for reason in &other.reasons {
            self.reject(reason.clone());
        }
        self.deterministic &= other.deterministic;
        self.may_fail |= other.may_fail;
    }

    /// Records a conservative rejection once while retaining resource evidence.
    pub(crate) fn reject(&mut self, reason: SequentialReason) {
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }

    /// Reports deterministic computation with no unmodeled or external effects.
    pub fn is_pure(&self) -> bool {
        self.deterministic && self.external_effects.is_empty() && self.reasons.is_empty()
    }
}

/// The dependency direction between different logical iterations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyKind {
    ReadAfterWrite,
    WriteAfterRead,
    WriteAfterWrite,
}

/// Evidence that accesses in different iterations may overlap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopCarriedDependency {
    pub kind: DependencyKind,
    pub source: ResourceAccess,
    pub destination: ResourceAccess,
    pub reason: SequentialReason,
}

/// Static safety only; runtime decides profitability and guards range/configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parallelism {
    Sequential {
        reasons: Vec<SequentialReason>,
    },
    /// Workers must journal writes and replay in logical iteration/source order.
    IndependentIterations {
        written_array_slots: Vec<LocalVariableSlot>,
        capture_read_slots: Vec<LocalVariableSlot>,
    },
}

/// One resolved loop body and its independence proof or rejection evidence.
#[derive(Clone, Debug)]
pub struct LoopAnalysis {
    pub span: Span,
    /// Static work for one logical iteration, including range bookkeeping.
    pub cost_per_iteration: super::WorkCost,
    pub effects: EffectSummary,
    pub parallelism: Parallelism,
    pub dependencies: Vec<LoopCarriedDependency>,
}

/// One compositional lexical region; pure means no mutation outside owned slots.
#[derive(Clone, Debug)]
pub struct BlockAnalysis {
    pub span: Span,
    pub effects: EffectSummary,
    pub pure: bool,
}

#[cfg(test)]
#[path = "effects.tests.rs"]
mod tests;
