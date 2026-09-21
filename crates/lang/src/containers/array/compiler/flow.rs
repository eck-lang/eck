//! The array element flow state and the operations merged across a program.

use crate::ir::{BindingId, TypedExpression, TypedExpressionKind};
use crate::semantic::SemanticType;
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
/// A slot holds `Some` for exact or finite semantic alternatives and `None` only
/// when its identity is genuinely open. The separate whole-array domain remains
/// available when a mutation loses positional knowledge, so finite observations
/// survive loop joins and dynamic-index writes without becoming contracts.
///
/// Every control-flow join in the compiler is expressed as an operation on this
/// state, so a branch, a loop pass, and a loop exit each merge through the same
/// type instead of rebuilding the map themselves.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct ArrayElementFlow {
    records: HashMap<BindingId, Vec<Option<SemanticType>>>,
    known_domains: HashMap<BindingId, Vec<SemanticType>>,
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

    /// Joins element states into the state every path agrees on.
    ///
    /// The join is the analysis' meet operation on one abstract flow state. A
    /// binding survives only when every state records it with the same length,
    /// and an element keeps its complete type only when every state records that
    /// exact type. Differing finite alternatives form a semantic union; an open
    /// input remains open, so a later read chooses finite or open dispatch from
    /// current knowledge rather than from the array contract.
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
                .map(|(index, _element)| {
                    let candidates = recorded
                        .iter()
                        .map(|other| other[index].clone())
                        .collect::<Option<Vec<_>>>();
                    candidates.map(SemanticType::union)
                })
                .collect();
            merged.insert(*binding, joined);
        }
        let mut known_domains = HashMap::new();
        for binding in first.known_domains.keys() {
            let domains = snapshots
                .iter()
                .filter_map(|snapshot| snapshot.known_domains.get(binding))
                .collect::<Vec<_>>();
            if domains.len() == snapshots.len() {
                known_domains.insert(
                    *binding,
                    flatten_semantic_types(domains.into_iter().flatten().cloned()),
                );
            }
        }
        Self {
            records: merged,
            known_domains,
        }
    }

    /// Drops every recorded element type.
    pub(crate) fn clear(&mut self) {
        self.records.clear();
        self.known_domains.clear();
    }

    /// Drops the record of one binding.
    pub(crate) fn remove(&mut self, binding: BindingId) {
        self.records.remove(&binding);
        self.known_domains.remove(&binding);
    }

    /// Stores the element slots of one binding.
    pub(crate) fn record(&mut self, binding: BindingId, element_types: Vec<Option<SemanticType>>) {
        let known = flatten_semantic_types(element_types.iter().filter_map(Clone::clone));
        self.records.insert(binding, element_types);
        self.known_domains.insert(binding, known);
    }

    /// Borrows the element slots recorded for one binding.
    pub(crate) fn element_types(&self, binding: BindingId) -> Option<Vec<Option<SemanticType>>> {
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
    ) -> Option<Option<SemanticType>> {
        self.records
            .get(&binding)?
            .get(index)
            .cloned()
            .or(Some(None))
    }

    /// Updates one constant element slot after a write.
    ///
    /// `None` records a genuinely open slot. An index outside the recorded range,
    /// or a binding without a positional record, leaves the record as it is.
    pub(crate) fn update_element(
        &mut self,
        binding: BindingId,
        index: usize,
        element_slot: Option<SemanticType>,
    ) {
        if let Some(element_types) = self.records.get_mut(&binding)
            && index < element_types.len()
        {
            element_types[index] = element_slot;
            self.refresh_known_domain(binding);
        }
    }

    /// Forgets slot positions while preserving the finite set any element may hold.
    pub(crate) fn update_unknown_index(
        &mut self,
        binding: BindingId,
        element_types: Vec<SemanticType>,
    ) {
        self.records.remove(&binding);
        if let Some(known) = self.known_domains.get_mut(&binding) {
            known.extend(element_types);
            *known = flatten_semantic_types(std::mem::take(known));
        }
    }

    /// Adds one known element at the selected end of a recorded array.
    pub(crate) fn insert_end(
        &mut self,
        binding: BindingId,
        at_front: bool,
        element: Option<SemanticType>,
    ) {
        if let Some(elements) = self.records.get_mut(&binding) {
            if at_front {
                elements.insert(0, element);
            } else {
                elements.push(element);
            }
            self.refresh_known_domain(binding);
        } else if let Some(element) = element {
            let known = self.known_domains.entry(binding).or_default();
            known.extend(flatten_semantic_types([element]));
            *known = flatten_semantic_types(std::mem::take(known));
        }
    }

    /// Removes one recorded element from the selected end.
    pub(crate) fn remove_end(&mut self, binding: BindingId, at_front: bool) {
        let Some(elements) = self.records.get_mut(&binding) else {
            return;
        };
        if elements.is_empty() {
            return;
        }
        if at_front {
            elements.remove(0);
        } else {
            elements.pop();
        }
        self.refresh_known_domain(binding);
    }

    /// Returns the finite set of currently recorded concrete element types.
    pub(crate) fn known_types(&self, binding: BindingId) -> Option<Vec<SemanticType>> {
        self.known_domains.get(&binding).cloned()
    }

    /// Rebuilds one binding's finite domain after a positional mutation.
    fn refresh_known_domain(&mut self, binding: BindingId) {
        let Some(elements) = self.records.get(&binding) else {
            return;
        };
        let known = flatten_semantic_types(elements.iter().filter_map(Clone::clone));
        self.known_domains.insert(binding, known);
    }
}

/// Flattens structural unions and removes duplicate semantic alternatives.
fn flatten_semantic_types(types: impl IntoIterator<Item = SemanticType>) -> Vec<SemanticType> {
    let mut flattened = Vec::new();
    for semantic_type in types {
        match semantic_type {
            SemanticType::Union(members) => flattened.extend(members.iter().cloned()),
            other => flattened.push(other),
        }
    }
    if flattened.is_empty() {
        return flattened;
    }
    match SemanticType::union(flattened) {
        SemanticType::Union(members) => members.iter().cloned().collect(),
        single => vec![single],
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
                let element_types: Vec<Option<SemanticType>> = elements
                    .iter()
                    .map(|element| self.array_flow_type_for_expression(element))
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
    ) -> Option<Option<SemanticType>> {
        let TypedExpressionKind::Variable { binding, .. } = &array.kind else {
            return None;
        };
        let index = constant_index?;
        self.element_flow.element_type(*binding, index)
    }

    /// Returns the finite element types currently known for one array binding.
    pub(crate) fn array_known_element_types(
        &self,
        array: &TypedExpression,
    ) -> Option<Vec<SemanticType>> {
        let TypedExpressionKind::Variable { binding, .. } = &array.kind else {
            return None;
        };
        self.element_flow.known_types(*binding)
    }

    /// Updates one recorded element slot after a constant element write.
    pub(crate) fn update_array_element_type(
        &mut self,
        binding: BindingId,
        index: usize,
        element_slot: Option<SemanticType>,
    ) {
        self.element_flow
            .update_element(binding, index, element_slot);
    }

    /// Preserves every finite runtime alternative an expression can produce.
    pub(crate) fn array_flow_type_for_expression(
        &self,
        expression: &TypedExpression,
    ) -> Option<SemanticType> {
        expression
            .complete_type_domain()
            .map(|domain| {
                SemanticType::union(domain.candidates.iter().copied().map(SemanticType::Scalar))
            })
            .or_else(|| expression.output.clone())
    }

    /// Updates a dynamic-index write without discarding the array's finite domain.
    pub(crate) fn update_array_unknown_index_type(
        &mut self,
        binding: BindingId,
        expression: &TypedExpression,
    ) {
        let types = self
            .array_flow_type_for_expression(expression)
            .into_iter()
            .flat_map(|semantic_type| match semantic_type {
                SemanticType::Union(members) => members.iter().cloned().collect(),
                other => vec![other],
            })
            .collect();
        self.element_flow.update_unknown_index(binding, types);
    }
}
