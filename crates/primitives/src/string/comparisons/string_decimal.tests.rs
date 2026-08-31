use crate::DecimalExtension;
use language_core::{Extension, Registry};

use super::*;
use crate::string::comparisons::test_support::{assert_distinct_equality, register_string_type};

/// Verifies strict equality semantics between strings and decimals.
#[test]
fn compares_strings_and_decimals_for_equality_only() {
    let mut registry = Registry::new();
    register_string_type(&mut registry);
    DecimalExtension.register(&mut registry).unwrap();
    register(&mut registry).unwrap();
    assert_distinct_equality(&registry, "decimal");
}
