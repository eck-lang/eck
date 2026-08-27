//! Native function registration and overload resolution.

use crate::{CoreError, FunctionDescriptor, FunctionId, FunctionSignature, NativeFunction, TypeId};

use super::Registry;

impl Registry {
    /// Registers a native function overload and returns its stable dense ID.
    ///
    /// Multiple overloads may share a name, but each signature may be
    /// registered only once. Exact signatures always take precedence over an
    /// [`FunctionSignature::AnySingle`] fallback, independently of extension
    /// registration order. Every referenced input and output type must already
    /// be registered.
    ///
    /// Returns [`CoreError::UnknownTypeId`] when a signature or output refers to
    /// an unknown type, or [`CoreError::DuplicateFunctionSignature`] when the
    /// same name and signature are already registered.
    pub fn register_function(
        &mut self,
        name: &'static str,
        signature: FunctionSignature,
        output: Option<TypeId>,
        execute: NativeFunction,
    ) -> Result<FunctionId, CoreError> {
        if let FunctionSignature::Exact(types) = &signature {
            for type_id in types {
                self.type_descriptor(*type_id)?;
            }
        }
        if let Some(type_id) = output {
            self.type_descriptor(type_id)?;
        }
        if self.functions_by_name.get(name).is_some_and(|candidates| {
            candidates.iter().any(|id| {
                self.functions
                    .get(id.index)
                    .is_some_and(|function| function.signature == signature)
            })
        }) {
            return Err(CoreError::DuplicateFunctionSignature {
                name: name.to_string(),
                signature: self.function_signature_name(&signature),
            });
        }

        let id = FunctionId {
            registry_id: self.registry_id,
            index: self.functions.len(),
        };
        self.functions.push(FunctionDescriptor {
            id,
            name,
            signature,
            output,
            execute,
        });
        self.functions_by_name.entry(name).or_default().push(id);
        Ok(id)
    }

    /// Resolves a function call by name and exact argument base types.
    ///
    /// The lookup first narrows candidates by name, then prefers an exact
    /// signature before considering an [`FunctionSignature::AnySingle`]
    /// fallback. Subtypes are intentionally ignored here; functions currently
    /// dispatch on base types only. Registration order never changes the
    /// selected overload.
    pub fn resolve_function(
        &self,
        name: &str,
        arg_types: &[TypeId],
    ) -> Result<FunctionId, CoreError> {
        let candidates = self
            .functions_by_name
            .get(name)
            .ok_or_else(|| CoreError::UnknownFunction(name.to_string()))?;

        for id in candidates {
            let function = &self.functions[id.index];
            if matches!(
                &function.signature,
                FunctionSignature::Exact(types) if types.as_slice() == arg_types
            ) {
                return Ok(*id);
            }
        }

        if arg_types.len() == 1
            && let Some(id) = candidates.iter().find(|id| {
                matches!(
                    self.functions[id.index].signature,
                    FunctionSignature::AnySingle
                )
            })
        {
            return Ok(*id);
        }

        Err(CoreError::NoMatchingFunction {
            name: name.to_string(),
            arguments: arg_types
                .iter()
                .map(|id| self.type_name(*id).to_string())
                .collect(),
        })
    }

    /// Returns the descriptor identified by a previously resolved function ID.
    ///
    /// Returns [`CoreError::UnknownFunctionId`] when `id` was resolved by
    /// another registry or is otherwise invalid.
    pub fn function(&self, id: FunctionId) -> Result<&FunctionDescriptor, CoreError> {
        if id.registry_id != self.registry_id {
            return Err(CoreError::UnknownFunctionId(id));
        }
        self.functions
            .get(id.index)
            .ok_or(CoreError::UnknownFunctionId(id))
    }

    /// Formats a registered function signature for deterministic diagnostics.
    fn function_signature_name(&self, signature: &FunctionSignature) -> String {
        match signature {
            FunctionSignature::Exact(types) => format!(
                "({})",
                types
                    .iter()
                    .map(|type_id| self.type_name(*type_id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FunctionSignature::AnySingle => "(any)".to_string(),
        }
    }
}
