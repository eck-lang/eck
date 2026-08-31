use super::*;
use language_core::{BinaryOperator, ComparisonOperator, Extension, Registry, Scale, ValueType};
use primitives::BoolExtension;
use primitives::DecimalExtension;
use primitives::IntegerExtension;

/// Builds a registry with time measures registered.
fn registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    TimeMeasureExtension.register(&mut registry).unwrap();
    registry
}

/// Verifies that the time extension registers every unit with its canonical suffix.
#[test]
fn registers_the_time_measure_extension_and_canonical_suffixes() {
    let registry = registry();

    assert_eq!(TimeMeasureExtension.name(), "time-measure");
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

    for suffix in ["sec", "hr", "mins", "nsec"] {
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
    let nanosecond = registry.subtype_by_suffix("ns").unwrap();
    let hour = registry.subtype_by_suffix("h").unwrap();

    let conversion = registry
        .resolve_subtype_conversion(ValueType::qualified(int, nanosecond), hour)
        .unwrap();

    assert_eq!(conversion.scale, Scale::new(1, 3_600_000_000_000));
    assert_eq!(conversion.output, ValueType::qualified(decimal, hour));
}

/// Verifies arithmetic for same units, mixed units and plain scalars.
#[test]
fn time_arithmetic_supports_same_and_mixed_units_and_plain_scalars() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let second = registry.subtype_by_suffix("s").unwrap();
    let millisecond = registry.subtype_by_suffix("ms").unwrap();

    for operator in [
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
        BinaryOperator::Remainder,
    ] {
        let same_unit = registry
            .resolve_binary_operation(
                operator,
                ValueType::qualified(int, second),
                ValueType::qualified(int, second),
            )
            .unwrap();
        assert_eq!(same_unit.output, ValueType::qualified(int, second));

        let mixed_units = registry
            .resolve_binary_operation(
                operator,
                ValueType::qualified(int, second),
                ValueType::qualified(int, millisecond),
            )
            .unwrap();
        assert_eq!(mixed_units.output, ValueType::qualified(int, millisecond));
        assert_eq!(mixed_units.left_operand_scale, Scale::integer(1_000));
        assert_eq!(mixed_units.right_operand_scale, Scale::IDENTITY);
    }

    for (operator, left, right) in [
        (
            BinaryOperator::Multiplication,
            ValueType::qualified(int, second),
            ValueType::plain(int),
        ),
        (
            BinaryOperator::Multiplication,
            ValueType::plain(int),
            ValueType::qualified(int, second),
        ),
        (
            BinaryOperator::Division,
            ValueType::qualified(int, second),
            ValueType::plain(int),
        ),
    ] {
        let resolution = registry
            .resolve_binary_operation(operator, left, right)
            .unwrap();
        assert_eq!(resolution.output, ValueType::qualified(int, second));
    }

    let ratio = registry
        .resolve_binary_operation(
            BinaryOperator::Division,
            ValueType::qualified(int, second),
            ValueType::qualified(int, millisecond),
        )
        .unwrap();
    assert_eq!(ratio.output, ValueType::plain(int));
    assert_eq!(ratio.left_operand_scale, Scale::integer(1_000));
    assert_eq!(ratio.right_operand_scale, Scale::IDENTITY);
}

/// Verifies that multiplying two durations and mixing time with other dimensions are rejected.
#[test]
fn multiplying_two_durations_and_mixing_time_with_other_dimensions_are_rejected() {
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
    let int = registry.type_by_name("int").unwrap();
    let second = registry.subtype_by_suffix("s").unwrap();
    let meter = registry.subtype_by_suffix("m").unwrap();
    let gram = registry.subtype_by_suffix("g").unwrap();
    let liter = registry.subtype_by_suffix("lt").unwrap();
    let hertz = registry.subtype_by_suffix("hz").unwrap();

    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Multiplication,
                ValueType::qualified(int, second),
                ValueType::qualified(int, second),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, second),
                ValueType::qualified(int, meter),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, second),
                ValueType::qualified(int, gram),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, second),
                ValueType::qualified(int, liter),
            )
            .is_err()
    );
    assert!(
        registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                ValueType::qualified(int, second),
                ValueType::qualified(int, hertz),
            )
            .is_err()
    );
}

/// Verifies that mixed time comparisons scale the coarser operand to the finer unit.
#[test]
fn mixed_time_comparisons_scale_to_the_finer_unit() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    TimeMeasureExtension.register(&mut registry).unwrap();
    let integer = registry.type_by_name("int").unwrap();
    let second = registry.subtype_by_suffix("s").unwrap();
    let millisecond = registry.subtype_by_suffix("ms").unwrap();
    let resolution = registry
        .resolve_comparison_operation(
            ComparisonOperator::Equal,
            ValueType::qualified(integer, second),
            ValueType::qualified(integer, millisecond),
        )
        .unwrap();
    assert_eq!(resolution.left_operand_scale, Scale::integer(1_000));
}
