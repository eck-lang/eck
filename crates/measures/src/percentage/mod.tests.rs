use language_core::{BinaryOperator, Extension, Registry, Scale, SubtypeDescriptor, ValueType};
use primitives::DecimalExtension;
use primitives::FloatExtension;
use primitives::IntegerExtension;

use super::PercentageMeasureExtension;

fn registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    PercentageMeasureExtension.register(&mut registry).unwrap();
    registry
}

/// Builds the numeric registry with an unrelated subtype installed before percentage.
///
/// The `unit` subtype stands in for an extension such as linear measures. Its
/// arithmetic semantics are intentionally absent: these tests isolate the
/// rules that percentage contributes when it composes with an existing subtype.
fn registry_with_unit() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    FloatExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();

    let unit = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id: unit,
            name: "unit",
            suffixes: &["u"],
        })
        .unwrap();
    PercentageMeasureExtension.register(&mut registry).unwrap();
    registry
}

#[test]
fn registers_percentage_name_and_canonical_suffix() {
    let registry = registry();

    let percentage = registry.subtype_by_name("percentage").unwrap();
    assert_eq!(registry.subtype_by_suffix("%"), Some(percentage));
    assert_eq!(
        registry
            .subtype_descriptor(percentage)
            .unwrap()
            .canonical_suffix(),
        "%"
    );
}

#[test]
fn multiplying_a_percentage_by_a_plain_number_scales_the_left_operand() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();

    for (name, result_name) in [
        ("int", "decimal"),
        ("float", "float"),
        ("decimal", "decimal"),
    ] {
        let number = registry.type_by_name(name).unwrap();
        let result = registry.type_by_name(result_name).unwrap();
        let resolution = registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(number, percentage),
                ValueType::plain(number),
            )
            .unwrap();

        assert_eq!(resolution.output, ValueType::plain(result));
        assert_eq!(resolution.left_operand_scale, Scale::new(1, 100));
        assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
    }
}

#[test]
fn multiplying_a_plain_number_by_a_percentage_scales_the_right_operand() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();

    for (name, result_name) in [
        ("int", "decimal"),
        ("float", "float"),
        ("decimal", "decimal"),
    ] {
        let number = registry.type_by_name(name).unwrap();
        let result = registry.type_by_name(result_name).unwrap();
        let resolution = registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::plain(number),
                ValueType::qualified(number, percentage),
            )
            .unwrap();

        assert_eq!(resolution.output, ValueType::plain(result));
        assert_eq!(resolution.left_operand_scale, Scale::IDENTITY);
        assert_eq!(resolution.right_operand_scale, Scale::new(1, 100));
    }
}

#[test]
fn multiplying_a_qualified_value_by_a_percentage_preserves_its_subtype() {
    let registry = registry_with_unit();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let unit = registry.subtype_by_name("unit").unwrap();

    for (name, result_name) in [
        ("int", "decimal"),
        ("float", "float"),
        ("decimal", "decimal"),
    ] {
        let number = registry.type_by_name(name).unwrap();
        let result = registry.type_by_name(result_name).unwrap();
        let resolution = registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(number, unit),
                ValueType::qualified(number, percentage),
            )
            .unwrap();

        assert_eq!(resolution.output, ValueType::qualified(result, unit));
        assert_eq!(resolution.left_operand_scale, Scale::IDENTITY);
        assert_eq!(resolution.right_operand_scale, Scale::new(1, 100));
    }
}

#[test]
fn multiplying_a_percentage_by_a_qualified_value_preserves_its_subtype() {
    let registry = registry_with_unit();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let unit = registry.subtype_by_name("unit").unwrap();

    for (name, result_name) in [
        ("int", "decimal"),
        ("float", "float"),
        ("decimal", "decimal"),
    ] {
        let number = registry.type_by_name(name).unwrap();
        let result = registry.type_by_name(result_name).unwrap();
        let resolution = registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(number, percentage),
                ValueType::qualified(number, unit),
            )
            .unwrap();

        assert_eq!(resolution.output, ValueType::qualified(result, unit));
        assert_eq!(resolution.left_operand_scale, Scale::new(1, 100));
        assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
    }
}

#[test]
fn multiplying_an_integer_unit_by_a_float_percentage_returns_a_float_unit() {
    let registry = registry_with_unit();
    let integer = registry.type_by_name("int").unwrap();
    let float = registry.type_by_name("float").unwrap();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let unit = registry.subtype_by_name("unit").unwrap();

    let resolution = registry
        .resolve_binary_operation(
            BinaryOperator::Multiplication,
            ValueType::qualified(integer, unit),
            ValueType::qualified(float, percentage),
        )
        .unwrap();

    assert_eq!(resolution.output, ValueType::qualified(float, unit));
    assert_eq!(resolution.left_operand_scale, Scale::IDENTITY);
    assert_eq!(resolution.right_operand_scale, Scale::new(1, 100));
}
