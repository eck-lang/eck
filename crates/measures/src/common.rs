use language_core::{
    BinaryOperator, ComparisonOperator, CoreError, Registry, Scale, SubtypeBinaryRule,
    SubtypeComparisonRule, SubtypeDescriptor, SubtypeId,
};

/// Describes a single metric unit within a dimensional measure.
pub(crate) struct UnitDefinition {
    pub(crate) name: &'static str,
    pub(crate) suffixes: &'static [&'static str],
    pub(crate) units_per_smallest: u64,
}

const COMPARISON_OPERATORS: &[ComparisonOperator] = &[
    ComparisonOperator::Equal,
    ComparisonOperator::NotEqual,
    ComparisonOperator::Less,
    ComparisonOperator::LessOrEqual,
    ComparisonOperator::Greater,
    ComparisonOperator::GreaterOrEqual,
];

/// Registers a subtype and returns its allocated identifier.
pub(crate) fn register_subtype(
    registry: &mut Registry,
    name: &'static str,
    suffixes: &'static [&'static str],
) -> Result<SubtypeId, CoreError> {
    let id = registry.allocate_subtype_id();
    registry.register_subtype(SubtypeDescriptor { id, name, suffixes })?;
    Ok(id)
}

/// Registers arithmetic rules for a single unit with itself and with plain scalars.
pub(crate) fn register_same_unit_arithmetic(
    registry: &mut Registry,
    unit: SubtypeId,
) -> Result<(), CoreError> {
    for operator in [
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
        BinaryOperator::Remainder,
    ] {
        registry.register_subtype_binary_rule(
            operator,
            Some(unit),
            Some(unit),
            SubtypeBinaryRule::new(Some(unit)),
        )?;
    }
    registry.register_subtype_binary_rule(
        BinaryOperator::Division,
        Some(unit),
        Some(unit),
        SubtypeBinaryRule::new(None),
    )?;
    Ok(())
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

/// Builds a reduced scale representing `numerator / denominator`.
fn reduced_scale(numerator: u64, denominator: u64) -> Scale {
    let divisor = greatest_common_divisor(numerator, denominator);
    Scale::new(numerator / divisor, denominator / divisor)
}

/// Registers arithmetic rules between two compatible units.
pub(crate) fn register_mixed_unit_arithmetic(
    registry: &mut Registry,
    coarser: SubtypeId,
    finer: SubtypeId,
    coarser_to_finer: Scale,
) -> Result<(), CoreError> {
    for operator in [
        BinaryOperator::Addition,
        BinaryOperator::Subtraction,
        BinaryOperator::Remainder,
    ] {
        registry.register_subtype_binary_rule(
            operator,
            Some(coarser),
            Some(finer),
            SubtypeBinaryRule::new(Some(finer))
                .with_operand_scales(coarser_to_finer, Scale::IDENTITY),
        )?;
        registry.register_subtype_binary_rule(
            operator,
            Some(finer),
            Some(coarser),
            SubtypeBinaryRule::new(Some(finer))
                .with_operand_scales(Scale::IDENTITY, coarser_to_finer),
        )?;
    }

    registry.register_subtype_binary_rule(
        BinaryOperator::Division,
        Some(coarser),
        Some(finer),
        SubtypeBinaryRule::new(None).with_operand_scales(coarser_to_finer, Scale::IDENTITY),
    )?;
    registry.register_subtype_binary_rule(
        BinaryOperator::Division,
        Some(finer),
        Some(coarser),
        SubtypeBinaryRule::new(None).with_operand_scales(Scale::IDENTITY, coarser_to_finer),
    )?;
    Ok(())
}

/// Registers scalar multiplication and division rules for a unit.
pub(crate) fn register_scalar_arithmetic(
    registry: &mut Registry,
    unit: SubtypeId,
) -> Result<(), CoreError> {
    registry.register_subtype_binary_rule(
        BinaryOperator::Multiplication,
        Some(unit),
        None,
        SubtypeBinaryRule::new(Some(unit)),
    )?;
    registry.register_subtype_binary_rule(
        BinaryOperator::Multiplication,
        None,
        Some(unit),
        SubtypeBinaryRule::new(Some(unit)),
    )?;
    registry.register_subtype_binary_rule(
        BinaryOperator::Division,
        Some(unit),
        None,
        SubtypeBinaryRule::new(Some(unit)),
    )?;
    Ok(())
}

/// Registers comparison rules for a single unit with itself.
pub(crate) fn register_same_unit_comparisons(
    registry: &mut Registry,
    unit: SubtypeId,
) -> Result<(), CoreError> {
    for operator in COMPARISON_OPERATORS {
        registry.register_subtype_comparison_rule(
            *operator,
            Some(unit),
            Some(unit),
            SubtypeComparisonRule::new(),
        )?;
    }
    Ok(())
}

/// Registers comparison rules between two compatible units.
pub(crate) fn register_mixed_unit_comparisons(
    registry: &mut Registry,
    coarser: SubtypeId,
    finer: SubtypeId,
    coarser_to_finer: Scale,
) -> Result<(), CoreError> {
    for operator in COMPARISON_OPERATORS {
        registry.register_subtype_comparison_rule(
            *operator,
            Some(coarser),
            Some(finer),
            SubtypeComparisonRule::new().with_operand_scales(coarser_to_finer, Scale::IDENTITY),
        )?;
        registry.register_subtype_comparison_rule(
            *operator,
            Some(finer),
            Some(coarser),
            SubtypeComparisonRule::new().with_operand_scales(Scale::IDENTITY, coarser_to_finer),
        )?;
    }
    Ok(())
}

/// Registers a full metric dimension (same-unit, mixed-unit and scalar rules plus conversions).
pub(crate) fn register_dimension(
    registry: &mut Registry,
    units: &[UnitDefinition],
) -> Result<Vec<(SubtypeId, u64)>, CoreError> {
    let mut registered = Vec::with_capacity(units.len());
    for unit in units {
        let id = register_subtype(registry, unit.name, unit.suffixes)?;
        register_same_unit_arithmetic(registry, id)?;
        register_same_unit_comparisons(registry, id)?;
        register_scalar_arithmetic(registry, id)?;
        registered.push((id, unit.units_per_smallest));
    }

    for (index, &(coarser, coarser_scale)) in registered.iter().enumerate() {
        for &(finer, finer_scale) in &registered[index + 1..] {
            let coarser_to_finer = reduced_scale(coarser_scale, finer_scale);
            let finer_to_coarser = reduced_scale(finer_scale, coarser_scale);
            register_mixed_unit_arithmetic(registry, coarser, finer, coarser_to_finer)?;
            register_mixed_unit_comparisons(registry, coarser, finer, coarser_to_finer)?;
            registry.register_subtype_conversion(coarser, finer, coarser_to_finer)?;
            registry.register_subtype_conversion(finer, coarser, finer_to_coarser)?;
        }
    }

    Ok(registered)
}
