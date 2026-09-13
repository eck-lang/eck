mod functions;

use language_core::{CoreError, Extension, FunctionSignature, Registry};

pub struct IoExtension;

impl Extension for IoExtension {
    fn name(&self) -> &'static str {
        "io"
    }

    fn register(&self, registry: &mut Registry) -> Result<(), CoreError> {
        registry.register_global_function(
            "print",
            FunctionSignature::AnySingle,
            None,
            functions::print,
        )?;
        Ok(())
    }
}
