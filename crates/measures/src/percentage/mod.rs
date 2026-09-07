use language_core::{
    BinaryOperator, CoreError, Extension, Registry, Scale, SubtypeBinaryRule, SubtypeDescriptor,
};

/// Registers percentage literals and their scalar multiplication rules.
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

        for subtype in existing_subtypes {
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
