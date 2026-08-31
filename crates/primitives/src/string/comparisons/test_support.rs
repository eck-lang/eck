use language_core::{ComparisonOperator, CoreError, Registry, TypeDescriptor, Value};

/// Registers the minimal string descriptor needed by isolated relation tests.
pub(super) fn register_string_type(registry: &mut Registry) {
    let string_type = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: string_type,
            name: "string",
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
}

/// Verifies strict equality in both orders and rejection of heterogeneous ordering.
pub(super) fn assert_distinct_equality(registry: &Registry, numeric_type_name: &str) {
    let string_type = registry.type_by_name("string").unwrap();
    let numeric_type = registry.type_by_name(numeric_type_name).unwrap();
    for (left_operand_type, right_operand_type) in
        [(string_type, numeric_type), (numeric_type, string_type)]
    {
        for operator in [ComparisonOperator::Equal, ComparisonOperator::NotEqual] {
            assert!(
                registry
                    .resolve_comparison(operator, left_operand_type, right_operand_type)
                    .is_ok()
            );
        }
        assert!(matches!(
            registry.resolve_comparison(
                ComparisonOperator::Less,
                left_operand_type,
                right_operand_type,
            ),
            Err(CoreError::ComparisonNotDefined { .. })
        ));
    }
}

/// Supplies an inert formatter required by the isolated string descriptor.
fn format_value(_: &Value) -> Result<String, CoreError> {
    Ok(String::new())
}
