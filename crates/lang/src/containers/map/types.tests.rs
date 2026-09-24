use std::collections::HashSet;

use super::*;
use crate::semantic::{Registry, ValueType};

/// Verifies that independently created dynamic contracts have one identity.
#[test]
fn dynamic_contracts_are_equal_and_hashable() {
    let mut contracts = HashSet::new();
    contracts.insert(MapType::dynamic());

    assert!(contracts.contains(&MapType::dynamic()));
    assert_eq!(contracts.len(), 1);
}

/// Verifies static contracts retain recursive key and value structure.
#[test]
fn static_contracts_retain_recursive_entry_types() {
    let mut registry = Registry::new();
    let scalar = SemanticType::Scalar(ValueType::plain(registry.allocate_type_id()));
    let nested = SemanticType::map(MapType::static_entries(scalar.clone(), scalar.clone()));
    let map_type = MapType::static_entries(scalar.clone(), nested.clone());

    assert_eq!(map_type.static_entry_types(), Some((&scalar, &nested)));
}
