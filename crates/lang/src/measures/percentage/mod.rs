use crate::semantic::{
    BinaryOperator, CoreError, Extension, Registry, Scale, SubtypeBinaryRule, SubtypeDescriptor,
    SubtypeRelativeRule,
};

/// Registers percentage literals with their arithmetic rules.
///
/// Multiplication and division scale the percentage operand by hundredths,
/// same-subtype addition and subtraction preserve the percentage, and a
/// percentage on the right of addition or subtraction reads as a fraction of
/// the left operand.
pub struct PercentageMeasureExtension;

impl Extension for PercentageMeasureExtension {
    fn name(&self) -> &'static str {
        "percentage-measure"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        // Percentage composes with every subtype that is already available.
        // Take the snapshot before registering ourselves so the rules preserve
        // the other subtype rather than accidentally producing percentages.
        let existing_subtypes = registry.registered_subtype_ids().collect::<Vec<_>>();
        let percentage = registry.allocate_subtype_id();
        registry.register_subtype(SubtypeDescriptor {
            id: percentage,
            name: "percentage",
            suffixes: &["%"],
        })?;
        comparisons::register(registry, percentage)?;

        registry.register_subtype_binary_rule(
            BinaryOperator::Multiplication,
            Some(percentage),
            None,
            SubtypeBinaryRule::new(None).with_operand_scales(Scale::new(1, 100), Scale::IDENTITY),
        )?;
        registry.register_subtype_binary_rule(
            BinaryOperator::Multiplication,
            None,
            Some(percentage),
            SubtypeBinaryRule::new(None).with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
        )?;

        registry.register_subtype_binary_rule(
            BinaryOperator::Division,
            Some(percentage),
            None,
            SubtypeBinaryRule::new(Some(percentage)),
        )?;
        registry.register_subtype_binary_rule(
            BinaryOperator::Division,
            None,
            Some(percentage),
            SubtypeBinaryRule::new(None).with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
        )?;
        registry.register_subtype_binary_rule(
            BinaryOperator::Division,
            Some(percentage),
            Some(percentage),
            SubtypeBinaryRule::new(None)
                .with_operand_scales(Scale::new(1, 100), Scale::new(1, 100)),
        )?;
        for operator in [BinaryOperator::Addition, BinaryOperator::Subtraction] {
            registry.register_subtype_binary_rule(
                operator,
                Some(percentage),
                Some(percentage),
                SubtypeBinaryRule::new(Some(percentage)),
            )?;
            // A qualified right operand reads as a fraction of the left
            // operand, so `100 - 50%` subtracts half of 100. Only the right
            // operand may be a percentage: the fraction needs the left
            // magnitude as its reference.
            registry.register_subtype_relative_rule(
                operator,
                None,
                Some(percentage),
                SubtypeRelativeRule::new(Scale::new(1, 100)),
            )?;
        }

        for subtype in existing_subtypes {
            for operator in [BinaryOperator::Addition, BinaryOperator::Subtraction] {
                registry.register_subtype_relative_rule(
                    operator,
                    Some(subtype),
                    Some(percentage),
                    SubtypeRelativeRule::new(Scale::new(1, 100)),
                )?;
            }
            registry.register_subtype_binary_rule(
                BinaryOperator::Division,
                Some(subtype),
                Some(percentage),
                SubtypeBinaryRule::new(Some(subtype))
                    .with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
            )?;
            registry.register_subtype_binary_rule(
                BinaryOperator::Multiplication,
                Some(subtype),
                Some(percentage),
                SubtypeBinaryRule::new(Some(subtype))
                    .with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
            )?;
            registry.register_subtype_binary_rule(
                BinaryOperator::Multiplication,
                Some(percentage),
                Some(subtype),
                SubtypeBinaryRule::new(Some(subtype))
                    .with_operand_scales(Scale::new(1, 100), Scale::IDENTITY),
            )?;
        }

        Ok(())
    }
}

mod comparisons;
#[cfg(test)]
#[path = "mod.tests.rs"]
mod tests;
