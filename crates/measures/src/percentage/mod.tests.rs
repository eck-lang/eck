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
fn dividing_a_percentage_by_a_plain_number_preserves_percentage() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();

    for name in ["int", "float", "decimal"] {
        let number = registry.type_by_name(name).unwrap();
        let resolution = registry
            .resolve_binary_operation(
                BinaryOperator::Division,
                ValueType::qualified(number, percentage),
                ValueType::plain(number),
            )
            .unwrap();

        assert_eq!(resolution.output, ValueType::qualified(number, percentage));
        assert_eq!(resolution.left_operand_scale, Scale::IDENTITY);
        assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
    }
}

#[test]
fn dividing_a_plain_number_by_a_percentage_scales_the_divisor() {
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
                BinaryOperator::Division,
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
fn dividing_a_percentage_by_a_percentage_produces_a_plain_number() {
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
                BinaryOperator::Division,
                ValueType::qualified(number, percentage),
                ValueType::qualified(number, percentage),
            )
            .unwrap();

        assert_eq!(resolution.output, ValueType::plain(result));
        assert_eq!(resolution.left_operand_scale, Scale::new(1, 100));
        assert_eq!(resolution.right_operand_scale, Scale::new(1, 100));
    }
}

#[test]
fn dividing_a_qualified_value_by_a_percentage_preserves_its_subtype() {
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
                BinaryOperator::Division,
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
fn dividing_a_percentage_by_a_qualified_value_is_not_defined() {
    let registry = registry_with_unit();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let unit = registry.subtype_by_name("unit").unwrap();
    let int = registry.type_by_name("int").unwrap();

    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Division,
                ValueType::qualified(int, percentage),
                ValueType::qualified(int, unit),
            )
            .is_err()
    );
}

#[test]
fn adding_and_subtracting_percentages_preserves_percentage() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();

    for operator in [BinaryOperator::Addition, BinaryOperator::Subtraction] {
        for name in ["int", "float", "decimal"] {
            let number = registry.type_by_name(name).unwrap();
            let resolution = registry
                .resolve_binary_operation(
                    operator,
                    ValueType::qualified(number, percentage),
                    ValueType::qualified(number, percentage),
                )
                .unwrap();

            assert_eq!(resolution.output, ValueType::qualified(number, percentage));
            assert_eq!(resolution.left_operand_scale, Scale::IDENTITY);
            assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
        }
    }
}

#[test]
fn adding_a_percentage_to_a_plain_number_is_not_defined() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let int = registry.type_by_name("int").unwrap();

    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, percentage),
                ValueType::plain(int),
            )
            .is_err()
    );
}

#[test]
fn adding_a_plain_number_and_a_percentage_resolves_relatively() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();

    for operator in [BinaryOperator::Addition, BinaryOperator::Subtraction] {
        for (name, result_name) in [
            ("int", "decimal"),
            ("float", "float"),
            ("decimal", "decimal"),
        ] {
            let number = registry.type_by_name(name).unwrap();
            let result = registry.type_by_name(result_name).unwrap();
            let resolution = registry
                .resolve_subtype_relative_rule(
                    operator,
                    ValueType::plain(number),
                    ValueType::qualified(number, percentage),
                )
                .unwrap();

            assert_eq!(resolution.output, ValueType::plain(result));
            assert_eq!(resolution.relative_adjustment, Some(Scale::new(1, 100)));
        }
    }
}

#[test]
fn adding_a_qualified_value_and_a_percentage_preserves_its_subtype() {
    let registry = registry_with_unit();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let unit = registry.subtype_by_name("unit").unwrap();

    for operator in [BinaryOperator::Addition, BinaryOperator::Subtraction] {
        for (name, result_name) in [
            ("int", "decimal"),
            ("float", "float"),
            ("decimal", "decimal"),
        ] {
            let number = registry.type_by_name(name).unwrap();
            let result = registry.type_by_name(result_name).unwrap();
            let resolution = registry
                .resolve_subtype_relative_rule(
                    operator,
                    ValueType::qualified(number, unit),
                    ValueType::qualified(number, percentage),
                )
                .unwrap();

            assert_eq!(resolution.output, ValueType::qualified(result, unit));
            assert_eq!(resolution.relative_adjustment, Some(Scale::new(1, 100)));
        }
    }
}

#[test]
fn relative_addition_rejects_a_percentage_on_the_left() {
    let registry = registry();
    let percentage = registry.subtype_by_name("percentage").unwrap();
    let int = registry.type_by_name("int").unwrap();

    assert!(
        registry
            .resolve_subtype_relative_rule(
                BinaryOperator::Addition,
                ValueType::qualified(int, percentage),
                ValueType::plain(int),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::plain(int),
                ValueType::qualified(int, percentage),
            )
            .is_err()
    );
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
