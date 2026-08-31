use super::*;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, Scale, ValueType};
use primitives::DecimalExtension;
use primitives::IntegerExtension;

/// Builds a registry with linear measures registered.
fn registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    LinearMeasureExtension.register(&mut registry).unwrap();
    registry
}

/// Verifies mixed addition scales the coarser operand to the finer unit.
#[test]
fn mixed_addition_converts_meters_to_millimeters() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();
    let millimeter = registry.subtype_by_suffix("mm").unwrap();

    let resolution = registry
        .resolve_binary_operation(
            BinaryOperator::Addition,
            ValueType::qualified(int, meter),
            ValueType::qualified(int, millimeter),
        )
        .unwrap();

    assert_eq!(resolution.output, ValueType::qualified(int, millimeter));
    assert_eq!(resolution.left_operand_scale, Scale::integer(1_000));
    assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
}

/// Verifies that singular and plural suffixes resolve to the same subtype.
#[test]
fn singular_and_plural_suffixes_resolve_to_the_same_subtype() {
    let registry = registry();

    for unit in UNITS {
        let expected = registry.subtype_by_suffix(unit.suffixes[0]);
        assert!(expected.is_some());
        for suffix in unit.suffixes {
            assert_eq!(registry.subtype_by_suffix(suffix), expected);
        }
    }
}

/// Verifies that every metric pair converts to the finer unit with the expected scale.
#[test]
fn every_metric_pair_converts_to_the_finer_unit() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();

    for (index, coarser) in UNITS.iter().enumerate() {
        for finer in &UNITS[index + 1..] {
            let coarser_id = registry.subtype_by_suffix(coarser.suffixes[0]).unwrap();
            let finer_id = registry.subtype_by_suffix(finer.suffixes[0]).unwrap();
            let expected_scale =
                Scale::integer(coarser.units_per_smallest / finer.units_per_smallest);

            let resolution = registry
                .resolve_binary_operation(
                    BinaryOperator::Addition,
                    ValueType::qualified(int, coarser_id),
                    ValueType::qualified(int, finer_id),
                )
                .unwrap();

            assert_eq!(resolution.output, ValueType::qualified(int, finer_id));
            assert_eq!(resolution.left_operand_scale, expected_scale);
            assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
        }
    }
}

/// Verifies that dividing two lengths produces a plain number.
#[test]
fn dividing_lengths_produces_a_plain_number() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();

    let resolution = registry
        .resolve_binary_operation(
            BinaryOperator::Division,
            ValueType::qualified(int, meter),
            ValueType::qualified(int, meter),
        )
        .unwrap();

    assert_eq!(resolution.output, ValueType::plain(int));
}

/// Verifies that converting a finer integer to a coarser unit promotes the output to decimal.
#[test]
fn conversion_to_a_coarser_unit_promotes_integers_to_decimal() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let decimal = registry.type_by_name("decimal").unwrap();
    let millimeter = registry.subtype_by_suffix("mm").unwrap();
    let kilometer = registry.subtype_by_suffix("km").unwrap();

    let conversion = registry
        .resolve_subtype_conversion(ValueType::qualified(int, millimeter), kilometer)
        .unwrap();

    assert_eq!(conversion.scale, Scale::new(1, 1_000_000));
    assert_eq!(conversion.output, ValueType::qualified(decimal, kilometer));
}

/// Verifies that multiplying two lengths is not defined.
#[test]
fn multiplying_two_lengths_is_not_defined_yet() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();

    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(int, meter),
                ValueType::qualified(int, meter),
            )
            .is_err()
    );
}

/// Verifies that mixed linear comparisons scale the coarser operand to the finer unit.
#[test]
fn mixed_linear_comparisons_scale_to_the_finer_unit() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    primitives::BoolExtension.register(&mut registry).unwrap();
    LinearMeasureExtension.register(&mut registry).unwrap();
    let integer = registry.type_by_name("int").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();
    let centimeter = registry.subtype_by_suffix("cm").unwrap();
    let resolution = registry
        .resolve_comparison_operation(
            ComparisonOperator::Equal,
            ValueType::qualified(integer, meter),
            ValueType::qualified(integer, centimeter),
        )
        .unwrap();
    assert_eq!(resolution.left_operand_scale, Scale::integer(100));
    assert_eq!(
        resolution.output,
        ValueType::plain(registry.type_by_name("bool").unwrap())
    );
}
