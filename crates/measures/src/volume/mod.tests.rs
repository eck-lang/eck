use super::*;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, Scale, ValueType};
use primitives::BoolExtension;
use primitives::DecimalExtension;
use primitives::IntegerExtension;

/// Builds a registry with volume measures registered.
fn registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    VolumeMeasureExtension.register(&mut registry).unwrap();
    registry
}

/// Verifies that the volume extension registers every unit with its canonical suffix.
#[test]
fn registers_the_volume_measure_extension_and_canonical_suffixes() {
    let registry = registry();

    assert_eq!(VolumeMeasureExtension.name(), "volume-measure");
    for unit in UNITS {
        let expected = registry.subtype_by_suffix(unit.suffixes[0]);
        assert!(expected.is_some());
        assert_eq!(registry.subtype_by_name(unit.name), expected);
        assert_eq!(
            registry
                .subtype_descriptor(expected.unwrap())
                .unwrap()
                .canonical_suffix(),
            unit.suffixes[0]
        );
        for suffix in unit.suffixes {
            assert_eq!(registry.subtype_by_suffix(suffix), expected);
        }
    }

    for suffix in ["l", "kl", "hl", "dal", "dl", "cl", "ml", "litre", "litres"] {
        assert_eq!(registry.subtype_by_suffix(suffix), None);
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

/// Verifies that converting a finer integer to a coarser unit promotes the output to decimal.
#[test]
fn conversion_to_a_coarser_unit_promotes_integers_to_decimal() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let decimal = registry.type_by_name("decimal").unwrap();
    let milliliter = registry.subtype_by_suffix("mlt").unwrap();
    let kiloliter = registry.subtype_by_suffix("klt").unwrap();

    let conversion = registry
        .resolve_subtype_conversion(ValueType::qualified(int, milliliter), kiloliter)
        .unwrap();

    assert_eq!(conversion.scale, Scale::new(1, 1_000_000));
    assert_eq!(conversion.output, ValueType::qualified(decimal, kiloliter));
}

/// Verifies arithmetic for same units, mixed units and plain scalars.
#[test]
fn volume_arithmetic_supports_same_and_mixed_units_and_plain_scalars() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let liter = registry.subtype_by_suffix("lt").unwrap();
    let milliliter = registry.subtype_by_suffix("mlt").unwrap();

    for operator in [
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
        BinaryOperator::Remainder,
    ] {
        let same_unit = registry
            .resolve_binary_operation(
                operator,
                ValueType::qualified(int, liter),
                ValueType::qualified(int, liter),
            )
            .unwrap();
        assert_eq!(same_unit.output, ValueType::qualified(int, liter));

        let mixed_units = registry
            .resolve_binary_operation(
                operator,
                ValueType::qualified(int, liter),
                ValueType::qualified(int, milliliter),
            )
            .unwrap();
        assert_eq!(mixed_units.output, ValueType::qualified(int, milliliter));
        assert_eq!(mixed_units.left_operand_scale, Scale::integer(1_000));
        assert_eq!(mixed_units.right_operand_scale, Scale::IDENTITY);
    }

    for (operator, left, right) in [
        (
            BinaryOperator::Multiplication,
            ValueType::qualified(int, liter),
            ValueType::plain(int),
        ),
        (
            BinaryOperator::Multiplication,
            ValueType::plain(int),
            ValueType::qualified(int, liter),
        ),
        (
            BinaryOperator::Division,
            ValueType::qualified(int, liter),
            ValueType::plain(int),
        ),
    ] {
        let resolution = registry
            .resolve_binary_operation(operator, left, right)
            .unwrap();
        assert_eq!(resolution.output, ValueType::qualified(int, liter));
    }

    let ratio = registry
        .resolve_binary_operation(
            BinaryOperator::Division,
            ValueType::qualified(int, liter),
            ValueType::qualified(int, milliliter),
        )
        .unwrap();
    assert_eq!(ratio.output, ValueType::plain(int));
    assert_eq!(ratio.left_operand_scale, Scale::integer(1_000));
    assert_eq!(ratio.right_operand_scale, Scale::IDENTITY);
}

/// Verifies that multiplying two volumes and mixing volume with length are rejected.
#[test]
fn multiplying_two_volumes_and_mixing_volume_with_length_are_rejected() {
    let mut registry = registry();
    // Volume and length must be independent dimensions.
    crate::linear::LinearMeasureExtension
        .register(&mut registry)
        .unwrap();
    let int = registry.type_by_name("int").unwrap();
    let liter = registry.subtype_by_suffix("lt").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();

    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(int, liter),
                ValueType::qualified(int, liter),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, liter),
                ValueType::qualified(int, meter),
            )
            .is_err()
    );
}

/// Verifies that mixed volume comparisons scale the coarser operand to the finer unit.
#[test]
fn mixed_volume_comparisons_scale_to_the_finer_unit() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    VolumeMeasureExtension.register(&mut registry).unwrap();
    let integer = registry.type_by_name("int").unwrap();
    let liter = registry.subtype_by_suffix("lt").unwrap();
    let milliliter = registry.subtype_by_suffix("mlt").unwrap();
    let resolution = registry
        .resolve_comparison_operation(
            ComparisonOperator::Equal,
            ValueType::qualified(integer, liter),
            ValueType::qualified(integer, milliliter),
        )
        .unwrap();
    assert_eq!(resolution.left_operand_scale, Scale::integer(1_000));
}
