//! The array element flow state and the operations merged across a program.

use crate::ir::{BindingId, TypedExpression, TypedExpressionKind};
use crate::semantic::{SemanticType, ValueType};
use std::collections::HashMap;

use crate::compiler::Compiler;

/// Records the static type of each element of a literal-initialized array.
///
/// An unconstrained array keeps no element subtype in its contract, so a
/// constant element read such as `sizes[0]` would otherwise lose the stored
/// element's subtype. Recording the literal element types preserves that subtype
/// for constant reads, and a constant write updates one entry while a dynamic
/// write removes the record and falls back to the array contract.
///
/// A slot holds `Some` for an element whose complete type the compiler knows and
/// `None` for one whose subtype is only known at runtime, so a read can tell a
/// precise element type from a dynamic one.
///
/// Every control-flow join in the compiler is expressed as an operation on this
/// state, so a branch, a loop pass, and a loop exit each merge through the same
/// type instead of rebuilding the map themselves.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct ArrayElementFlow {
    records: HashMap<BindingId, Vec<Option<ValueType>>>,
}

impl ArrayElementFlow {
    /// Returns the empty state a compilation starts from.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns a copy of the state before compiling a control-flow path.
    pub(crate) fn snapshot(&self) -> Self {
        self.clone()
    }

    /// Restores a state before compiling another control-flow path.
    pub(crate) fn restore(&mut self, snapshot: Self) {
        *self = snapshot;
    }

    /// Replaces the state with the join of every given path state.
    pub(crate) fn merge(&mut self, snapshots: &[Self]) {
        *self = Self::join(snapshots);
    }

    /// Joins element states into the state every path agrees on.
    ///
    /// The join is the analysis' meet operation on one abstract flow state. A
    /// binding survives only when every state records it with the same length,
    /// and an element keeps its complete type only when every state records that
    /// exact type. Every other element becomes dynamic, so a later read
    /// dispatches on the subtype the stored value actually holds.
    ///
    /// Returning the joined state instead of assigning it lets a loop compare the
    /// head of one iteration with the head of the next while looking for a fixed
    /// point.
    pub(crate) fn join(snapshots: &[Self]) -> Self {
        let Some(first) = snapshots.first() else {
            return Self::new();
        };
        let mut merged = HashMap::new();
        for (binding, elements) in &first.records {
            let recorded = snapshots
                .iter()
                .filter_map(|snapshot| snapshot.records.get(binding))
                .collect::<Vec<_>>();
            if recorded.len() != snapshots.len()
                || recorded.iter().any(|other| other.len() != elements.len())
            {
                continue;
            }
            let joined = elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    let agrees = recorded.iter().all(|other| other[index] == *element);
                    match agrees {
                        true => *element,
                        false => None,
                    }
                })
                .collect();
            merged.insert(*binding, joined);
        }
        Self { records: merged }
    }

    /// Drops every recorded element type.
    pub(crate) fn clear(&mut self) {
        self.records.clear();
    }

    /// Drops the record of one binding.
    pub(crate) fn remove(&mut self, binding: BindingId) {
        self.records.remove(&binding);
    }

    /// Stores the element slots of one binding.
    pub(crate) fn record(&mut self, binding: BindingId, element_types: Vec<Option<ValueType>>) {
        self.records.insert(binding, element_types);
    }

    /// Borrows the element slots recorded for one binding.
    pub(crate) fn element_types(&self, binding: BindingId) -> Option<Vec<Option<ValueType>>> {
        self.records.get(&binding).cloned()
    }

    /// Returns one constant element slot.
    ///
    /// The outer option reports whether the array still has a record at all: a
    /// miss means the whole array is dynamic. A present inner `None` means the
    /// slot itself is dynamic, while `Some` carries the element's static type.
    pub(crate) fn element_type(
        &self,
        binding: BindingId,
        index: usize,
    ) -> Option<Option<ValueType>> {
        self.records
            .get(&binding)?
            .get(index)
            .copied()
            .or(Some(None))
    }

    /// Updates one constant element slot after a write.
    ///
    /// `None` records a dynamic slot, because the written value's subtype is only
    /// known at runtime. An index outside the recorded range, or a binding without
    /// a record, leaves the record as it is; the read then falls back to
    /// dispatching on the stored subtype.
    pub(crate) fn update_element(
        &mut self,
        binding: BindingId,
        index: usize,
        element_slot: Option<ValueType>,
    ) {
        if let Some(element_types) = self.records.get_mut(&binding)
            && index < element_types.len()
        {
            element_types[index] = element_slot;
        }
    }
}

impl Compiler<'_> {
    /// Records the element slots of a freshly bound array.
    ///
    /// A literal initializer contributes each element's own type, which keeps an
    /// unconstrained array's per-element subtypes available to constant reads. An
    /// element whose type is only known at runtime records a dynamic slot, so a
    /// later read of it still dispatches on the subtype the value holds. An array
    /// copied from another binding inherits that binding's record so the alias
    /// behaves like its source, and any other initializer clears it.
    pub(crate) fn record_array_element_types(
        &mut self,
        binding: BindingId,
        expression: &TypedExpression,
    ) {
        match &expression.kind {
            TypedExpressionKind::ArrayLiteral { elements, .. } => {
                let element_types: Vec<Option<ValueType>> = elements
                    .iter()
                    .map(|element| {
                        if element.complete_type_domain().is_some() {
                            None
                        } else {
                            match element.output {
                                Some(SemanticType::Scalar(element_type)) => Some(element_type),
                                Some(SemanticType::Array(_)) | None => None,
                            }
                        }
                    })
                    .collect();
                self.element_flow.record(binding, element_types);
            }
            TypedExpressionKind::Variable {
                binding: source, ..
            } if matches!(expression.output, Some(SemanticType::Array(_))) => {
                match self.element_flow.element_types(*source) {
                    Some(element_types) => self.element_flow.record(binding, element_types),
                    None => self.element_flow.remove(binding),
                }
            }
            _ => {
                self.element_flow.remove(binding);
            }
        }
    }

    /// Returns the recorded element slot for one constant element read.
    pub(crate) fn array_element_type(
        &self,
        array: &TypedExpression,
        constant_index: Option<usize>,
    ) -> Option<Option<ValueType>> {
        let TypedExpressionKind::Variable { binding, .. } = &array.kind else {
            return None;
        };
        let index = constant_index?;
        self.element_flow.element_type(*binding, index)
    }

    /// Updates one recorded element slot after a constant element write.
    pub(crate) fn update_array_element_type(
        &mut self,
        binding: BindingId,
        index: usize,
        element_slot: Option<ValueType>,
    ) {
        self.element_flow
            .update_element(binding, index, element_slot);
    }
}
