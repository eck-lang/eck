mod functions;

use crate::semantic::{
    CoreError, Extension, FunctionDeterminism, FunctionEffectSummary, FunctionExternalEffect,
    FunctionPurity, FunctionSignature, Registry,
};

pub struct IoExtension;

impl Extension for IoExtension {
    fn name(&self) -> &'static str {
        "io"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        let print = registry.register_global_function_with_effect_summary(
            "print",
            FunctionSignature::AnySingle,
            None,
            functions::print,
            FunctionEffectSummary {
                purity: FunctionPurity::Impure,
                determinism: FunctionDeterminism::Deterministic,
                may_fail: true,
                external_effect: FunctionExternalEffect::WritesExternalState,
            },
        )?;
        registry.set_function_parameter_names(print, &["value"])?;
        Ok(())
    }
}
