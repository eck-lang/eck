//! Resolved execution effects and conservative range-iteration independence.
//!
//! This pass consumes typed IR once. Eligibility assumes worker-private locals,
//! indexed writes recorded in a journal, same-iteration reads consulting that
//! journal, and ordered replay including the successful write prefix of a
//! failing iteration. Scheduling and configuration guards belong to the runtime.

mod effects;
mod resolved;

pub use effects::*;

use std::collections::HashMap;

use crate::ir::TypedProgram;
use crate::semantic::Registry;
use crate::syntax::Span;

/// Owned metadata retained beside a resolved program, never recomputed per iteration.
#[derive(Clone, Debug, Default)]
pub struct ExecutionAnalysis {
    pub effects: EffectSummary,
    /// Loop entries keyed by source byte offsets because `Span` is not hashable.
    pub loops: HashMap<(usize, usize), LoopAnalysis>,
    /// Compositional block effects, including branch and loop bodies.
    pub blocks: Vec<BlockAnalysis>,
}

impl ExecutionAnalysis {
    /// Looks up a loop by its complete source span; duplicate spans are sequential.
    pub fn loop_analysis(&self, span: Span) -> Option<&LoopAnalysis> {
        self.loops.get(&(span.start, span.end))
    }
}

/// Computes execution effects and independence from resolved statements and slots.
pub fn analyze(program: &TypedProgram, registry: &Registry) -> ExecutionAnalysis {
    resolved::analyze(program, registry)
}

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
