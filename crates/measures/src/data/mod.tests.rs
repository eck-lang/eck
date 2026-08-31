use super::*;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, Scale, ValueType};
use primitives::BoolExtension;
use primitives::DecimalExtension;
use primitives::IntegerExtension;

/// Builds a registry with data measures registered.
fn registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    DataMeasureExtension.register(&mut registry).unwrap();
    registry
}

/// Verifies that the data extension registers every unit with its canonical suffix.
#[test]
fn registers_the_data_measure_extension_and_canonical_suffixes() {
    let registry = registry();

    assert_eq!(DataMeasureExtension.name(), "data-measure");
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

    for suffix in ["b", "kb", "mb", "gb", "tb", "pb", "bytes", "KBs"] {
        // Suffix matching is case-sensitive: only canonical capital forms are valid.
        if suffix == "bytes" {
            continue;
        }
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
    let byte = registry.subtype_by_suffix("B").unwrap();
    let gigabyte = registry.subtype_by_suffix("GB").unwrap();

    let conversion = registry
        .resolve_subtype_conversion(ValueType::qualified(int, byte), gigabyte)
        .unwrap();

    assert_eq!(conversion.scale, Scale::new(1, 1_000_000_000));
    assert_eq!(conversion.output, ValueType::qualified(decimal, gigabyte));
}

/// Verifies arithmetic for same units, mixed units and plain scalars.
#[test]
fn data_arithmetic_supports_same_and_mixed_units_and_plain_scalars() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let megabyte = registry.subtype_by_suffix("MB").unwrap();
    let kilobyte = registry.subtype_by_suffix("KB").unwrap();

    for operator in [
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
        BinaryOperator::Remainder,
    ] {
        let same_unit = registry
            .resolve_binary_operation(
                operator,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, megabyte),
            )
            .unwrap();
        assert_eq!(same_unit.output, ValueType::qualified(int, megabyte));

        let mixed_units = registry
            .resolve_binary_operation(
                operator,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, kilobyte),
            )
            .unwrap();
        assert_eq!(mixed_units.output, ValueType::qualified(int, kilobyte));
        assert_eq!(mixed_units.left_operand_scale, Scale::integer(1_000));
        assert_eq!(mixed_units.right_operand_scale, Scale::IDENTITY);
    }

    for (operator, left, right) in [
        (
            BinaryOperator::Multiplication,
            ValueType::qualified(int, megabyte),
            ValueType::plain(int),
        ),
        (
            BinaryOperator::Multiplication,
            ValueType::plain(int),
            ValueType::qualified(int, megabyte),
        ),
        (
            BinaryOperator::Division,
            ValueType::qualified(int, megabyte),
            ValueType::plain(int),
        ),
    ] {
        let resolution = registry
            .resolve_binary_operation(operator, left, right)
            .unwrap();
        assert_eq!(resolution.output, ValueType::qualified(int, megabyte));
    }

    let ratio = registry
        .resolve_binary_operation(
            BinaryOperator::Division,
            ValueType::qualified(int, megabyte),
            ValueType::qualified(int, kilobyte),
        )
        .unwrap();
    assert_eq!(ratio.output, ValueType::plain(int));
    assert_eq!(ratio.left_operand_scale, Scale::integer(1_000));
    assert_eq!(ratio.right_operand_scale, Scale::IDENTITY);
}

/// Verifies that multiplying two data sizes and mixing data with other dimensions are rejected.
#[test]
fn multiplying_two_data_sizes_and_mixing_data_with_other_dimensions_are_rejected() {
    let mut registry = registry();
    crate::linear::LinearMeasureExtension
        .register(&mut registry)
        .unwrap();
    crate::mass::MassMeasureExtension
        .register(&mut registry)
        .unwrap();
    crate::volume::VolumeMeasureExtension
        .register(&mut registry)
        .unwrap();
    crate::frequency::FrequencyMeasureExtension
        .register(&mut registry)
        .unwrap();
    crate::time::TimeMeasureExtension
        .register(&mut registry)
        .unwrap();
    let int = registry.type_by_name("int").unwrap();
    let megabyte = registry.subtype_by_suffix("MB").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();
    let gram = registry.subtype_by_suffix("g").unwrap();
    let liter = registry.subtype_by_suffix("lt").unwrap();
    let hertz = registry.subtype_by_suffix("hz").unwrap();
    let second = registry.subtype_by_suffix("s").unwrap();

    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, megabyte),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, meter),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, gram),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, liter),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, hertz),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, megabyte),
                ValueType::qualified(int, second),
            )
            .is_err()
    );
}

/// Verifies that mixed data comparisons scale the coarser operand to the finer unit.
#[test]
fn mixed_data_comparisons_scale_to_the_finer_unit() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    DataMeasureExtension.register(&mut registry).unwrap();
    let integer = registry.type_by_name("int").unwrap();
    let megabyte = registry.subtype_by_suffix("MB").unwrap();
    let kilobyte = registry.subtype_by_suffix("KB").unwrap();
    let resolution = registry
        .resolve_comparison_operation(
            ComparisonOperator::Equal,
            ValueType::qualified(integer, megabyte),
            ValueType::qualified(integer, kilobyte),
        )
        .unwrap();
    assert_eq!(resolution.left_operand_scale, Scale::integer(1_000));
}
