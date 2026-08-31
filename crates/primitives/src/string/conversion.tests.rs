use crate::IntegerExtension;
use language_core::{ExecutionContext, Extension, Registry};

use super::*;
use crate::StringExtension;

/// Verifies conversion through the source value's registered formatter.
#[test]
fn converts_formatted_values_to_strings() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    StringExtension.register(&mut registry).unwrap();
    let integer = registry.parse_numeric("125", None).unwrap();

    let configuration = registry.default_runtime_configuration();
    let context = ExecutionContext::new(&registry, &configuration);
    let result = format_as_string(&context, &[integer]).unwrap().unwrap();

    assert_eq!(result.downcast_ref::<String>().unwrap(), "125");
}
