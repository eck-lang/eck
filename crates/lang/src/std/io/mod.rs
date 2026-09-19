mod functions;

use crate::semantic::{CoreError, Extension, FunctionSignature, Registry};

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
