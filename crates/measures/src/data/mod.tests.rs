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

/// Returns the greatest common divisor of two values.
fn greatest_common_divisor(mut first: u64, mut second: u64) -> u64 {
    while second != 0 {
        let remainder = first % second;
        first = second;
        second = remainder;
    }
    first
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

    for suffix in [
        "kb", "mb", "gb", "tb", "pb", "kib", "mib", "gib", "tib", "pib", "KBs", "kB",
    ] {
        assert_eq!(registry.subtype_by_suffix(suffix), None);
    }
}

/// Verifies that every metric pair converts to the finer unit with the expected scale.
#[test]
fn every_metric_pair_converts_to_the_finer_unit() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let decimal = registry.type_by_name("decimal").unwrap();

    for (index, coarser) in UNITS.iter().enumerate() {
        for finer in &UNITS[index + 1..] {
            let coarser_id = registry.subtype_by_suffix(coarser.suffixes[0]).unwrap();
            let finer_id = registry.subtype_by_suffix(finer.suffixes[0]).unwrap();
            let divisor =
                greatest_common_divisor(coarser.units_per_smallest, finer.units_per_smallest);
            let expected_scale = Scale::new(
                coarser.units_per_smallest / divisor,
                finer.units_per_smallest / divisor,
            );
            let is_integer = coarser.units_per_smallest % finer.units_per_smallest == 0;
            let expected_output = if is_integer {
                ValueType::qualified(int, finer_id)
            } else {
                // Fractional scales require the default fractional type.
                ValueType::qualified(decimal, finer_id)
            };

            let resolution = registry
                .resolve_binary_operation(
                    BinaryOperator::Addition,
                    ValueType::qualified(int, coarser_id),
                    ValueType::qualified(int, finer_id),
                )
                .unwrap();

            assert_eq!(resolution.output, expected_output);
            assert_eq!(resolution.left_operand_scale, expected_scale);
            assert_eq!(resolution.right_operand_scale, Scale::IDENTITY);
        }
    }
}

/// Verifies that bit and byte have the 8:1 relation and that decimal/binary units interconvert.
#[test]
fn bit_byte_and_decimal_binary_interconversion() {
    let registry = registry();
    let int = registry.type_by_name("int").unwrap();
    let bit = registry.subtype_by_suffix("b").unwrap();
    let byte = registry.subtype_by_suffix("B").unwrap();
    let kilobyte = registry.subtype_by_suffix("KB").unwrap();
    let kibibyte = registry.subtype_by_suffix("KiB").unwrap();

    let bit_to_byte = registry
        .resolve_binary_operation(
            BinaryOperator::Addition,
            ValueType::qualified(int, byte),
            ValueType::qualified(int, bit),
        )
        .unwrap();
    assert_eq!(bit_to_byte.left_operand_scale, Scale::integer(8));

    // KiB (8192 bits) is coarser than KB (8000 bits): 8192/8000 =128/125
    let kib_to_kb = registry
        .resolve_binary_operation(
            BinaryOperator::Addition,
            ValueType::qualified(int, kibibyte),
            ValueType::qualified(int, kilobyte),
        )
        .unwrap();
    assert_eq!(kib_to_kb.left_operand_scale, Scale::new(128, 125));
    assert_eq!(kib_to_kb.right_operand_scale, Scale::IDENTITY);

    let kb_to_kib = registry
        .resolve_binary_operation(
            BinaryOperator::Addition,
            ValueType::qualified(int, kilobyte),
            ValueType::qualified(int, kibibyte),
        )
        .unwrap();
    assert_eq!(kb_to_kib.left_operand_scale, Scale::IDENTITY);
    assert_eq!(kb_to_kib.right_operand_scale, Scale::new(128, 125));
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

    let bit = registry.subtype_by_suffix("b").unwrap();
    let conversion_bit = registry
        .resolve_subtype_conversion(ValueType::qualified(int, bit), byte)
        .unwrap();
    assert_eq!(conversion_bit.scale, Scale::new(1, 8));
    assert_eq!(conversion_bit.output, ValueType::qualified(decimal, byte));
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
