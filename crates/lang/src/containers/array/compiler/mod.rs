//! Compilation of array element contracts, literals, and element access.
//!
//! An array's static type is its element contract: the declared or inferred
//! element base type, an optional constrained element subtype, and its element
//! representation mode. Every element is compiled against that contract here,
//! and the index conversion the runtime needs is resolved once so element access
//! never repeats registry work during execution.
//!
//! This module owns the array compilation protocol and delegates one concern to
//! each child module: declared and inferred types, the storage-boundary element
//! contract, element access, end operations, and the element flow state merged
//! across branches and loop passes. The entry points are inherent methods on the
//! compiler, so the compiler driver calls them wherever array syntax appears,
//! which is why the compiler exposes the services they consume.

mod access;
mod contract;
mod declaration;
mod end_operations;
mod flow;

pub(crate) use flow::ArrayElementFlow;

#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
