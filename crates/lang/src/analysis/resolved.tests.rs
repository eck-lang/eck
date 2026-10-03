use super::*;

/// Indexed disjointness requires the same resolved induction binding and nonzero slope.
#[test]
fn affine_disjointness_uses_resolved_identity() {
    let iteration = binding_resource(BindingId(1), LocalVariableSlot(1));
    let container = binding_resource(BindingId(2), LocalVariableSlot(2));
    let index = AffineIndex {
        iteration,
        coefficient: 2,
        offset: 1,
    };
    let first = Resource::ArrayElement {
        container,
        index: IndexAccess::Affine(index),
    };
    assert!(disjoint_iterations(&first, &first, iteration));
    let shifted = Resource::ArrayElement {
        container,
        index: IndexAccess::Affine(AffineIndex { offset: 2, ..index }),
    };
    assert!(!disjoint_iterations(&first, &shifted, iteration));
    let wrong_iteration = binding_resource(BindingId(3), iteration.slot);
    assert!(!disjoint_iterations(&first, &first, wrong_iteration));
    assert!(!disjoint_iterations(
        &first,
        &Resource::Binding(container),
        iteration
    ));
}
