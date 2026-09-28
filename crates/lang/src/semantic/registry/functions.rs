//! Native function registration and overload resolution.

use crate::semantic::{
    CoreError, FunctionDescriptor, FunctionId, FunctionSignature, NativeFunction, SemanticType,
    TypeId, Value, ValueType,
};

use super::Registry;

impl Registry {
    /// Registers a native function overload that source may call without an import.
    pub fn register_global_function(
        &mut self,
        name: &'static str,
        signature: FunctionSignature,
        output: Option<TypeId>,
        execute: NativeFunction,
    ) -> Result<FunctionId, CoreError> {
        self.register_global_function_with_output(
            name,
            signature,
            output.map(|type_id| SemanticType::Scalar(ValueType::plain(type_id))),
            execute,
        )
    }

    /// Registers a global native function with a scalar or structural output.
    pub fn register_global_function_with_output(
        &mut self,
        name: &'static str,
        signature: FunctionSignature,
        output: Option<SemanticType>,
        execute: NativeFunction,
    ) -> Result<FunctionId, CoreError> {
        let function = self.register_function_with_output(name, signature, output, execute)?;
        self.global_functions.insert(name);
        Ok(function)
    }

    /// Returns whether a native function family belongs to the small global surface.
    pub fn is_global_function(&self, name: &str) -> bool {
        self.global_functions.contains(name)
    }

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
        self.register_function_with_output(
            name,
            signature,
            output.map(|type_id| SemanticType::Scalar(ValueType::plain(type_id))),
            execute,
        )
    }

    /// Registers a native function whose output may have a structural semantic type.
    pub fn register_function_with_output(
        &mut self,
        name: &'static str,
        signature: FunctionSignature,
        output: Option<SemanticType>,
        execute: NativeFunction,
    ) -> Result<FunctionId, CoreError> {
        if let FunctionSignature::Exact(types) = &signature {
            for type_id in types {
                self.type_descriptor(*type_id)?;
            }
        }
        if let Some(SemanticType::Scalar(value_type)) = &output {
            self.type_descriptor(value_type.base)?;
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
        let parameter_count = match &signature {
            FunctionSignature::Exact(types) => types.len(),
            FunctionSignature::AnySingle => 1,
        };
        self.functions.push(FunctionDescriptor {
            id,
            name,
            signature,
            parameter_names: None,
            parameter_defaults: vec![None; parameter_count],
            output,
            execute,
        });
        self.functions_by_name.entry(name).or_default().push(id);
        Ok(id)
    }

    /// Attaches stable callback parameter names to an already registered overload.
    pub fn set_function_parameter_names(
        &mut self,
        id: FunctionId,
        names: &[&'static str],
    ) -> Result<(), CoreError> {
        let descriptor = self.function(id)?;
        let arity = match &descriptor.signature {
            FunctionSignature::Exact(types) => types.len(),
            FunctionSignature::AnySingle => 1,
        };
        if names.len() != arity {
            return Err(CoreError::InvalidFunctionParameterNames {
                name: descriptor.name.into(),
                message: format!("expected {arity} names, found {}", names.len()),
            });
        }
        for (index, name) in names.iter().enumerate() {
            if name.is_empty() || names[..index].contains(name) {
                return Err(CoreError::InvalidFunctionParameterNames {
                    name: descriptor.name.into(),
                    message: format!("empty or duplicate parameter name `{name}`"),
                });
            }
        }
        self.functions[id.index].parameter_names = Some(names.to_vec());
        Ok(())
    }

    /// Declares compile-time literal defaults in callback parameter order.
    ///
    /// A positional call may omit only a trailing run of defaults. Named calls
    /// may omit any parameter with a default, including one in the middle.
    pub fn set_function_parameter_defaults(
        &mut self,
        id: FunctionId,
        defaults: &[Option<Value>],
    ) -> Result<(), CoreError> {
        let descriptor = self.function(id)?;
        if defaults.len() != descriptor.parameter_defaults.len() {
            return Err(CoreError::InvalidFunctionParameterNames {
                name: descriptor.name.into(),
                message: format!(
                    "expected {} defaults, found {}",
                    descriptor.parameter_defaults.len(),
                    defaults.len()
                ),
            });
        }
        self.functions[id.index].parameter_defaults = defaults.to_vec();
        Ok(())
    }

    /// Resolves a named call and returns source argument indices or defaults in callback order.
    pub fn resolve_named_function(
        &self,
        name: &str,
        positional_types: &[Option<TypeId>],
        named_types: &[(&str, Option<TypeId>)],
    ) -> Result<(FunctionId, Vec<Option<usize>>), CoreError> {
        let candidates = self
            .functions_by_name
            .get(name)
            .ok_or_else(|| CoreError::UnknownFunction(name.into()))?;
        let named_candidates: Vec<_> = candidates
            .iter()
            .map(|id| &self.functions[id.index])
            .filter(|descriptor| descriptor.parameter_names.is_some())
            .collect();
        if named_candidates.is_empty() {
            return Err(CoreError::UnnamedFunctionParameters(name.into()));
        }
        for (argument_name, _) in named_types {
            if !named_candidates.iter().any(|descriptor| {
                descriptor
                    .parameter_names
                    .as_ref()
                    .is_some_and(|names| names.contains(argument_name))
            }) {
                return Err(CoreError::UnknownNamedArgument((*argument_name).into()));
            }
        }

        let mut duplicate = None;
        let mut missing = None;
        let mut fallback = None;
        for descriptor in named_candidates {
            let names = descriptor.parameter_names.as_ref().expect("filtered above");
            if positional_types.len() > names.len() {
                continue;
            }
            let mut order = vec![None; names.len()];
            for (index, slot) in order.iter_mut().enumerate().take(positional_types.len()) {
                *slot = Some(index);
            }
            let mut known_names = true;
            let mut candidate_duplicate = None;
            for (index, (argument_name, _)) in named_types.iter().enumerate() {
                let Some(parameter_index) = names.iter().position(|name| name == argument_name)
                else {
                    known_names = false;
                    break;
                };
                if order[parameter_index]
                    .replace(positional_types.len() + index)
                    .is_some()
                {
                    candidate_duplicate.get_or_insert((*argument_name).to_string());
                }
            }
            if !known_names {
                continue;
            }
            if let Some(name) = candidate_duplicate {
                duplicate.get_or_insert(name);
                continue;
            }
            if let Some(index) = order.iter().enumerate().position(|(index, source)| {
                source.is_none() && descriptor.parameter_defaults[index].is_none()
            }) {
                missing.get_or_insert(names[index].to_string());
                continue;
            }
            let matches = match &descriptor.signature {
                FunctionSignature::Exact(types) => {
                    types.iter().enumerate().all(|(index, expected)| {
                        let Some(source_index) = order[index] else {
                            return true;
                        };
                        let actual = if source_index < positional_types.len() {
                            positional_types[source_index]
                        } else {
                            named_types[source_index - positional_types.len()].1
                        };
                        actual == Some(*expected)
                    })
                }
                FunctionSignature::AnySingle => true,
            };
            if matches {
                if matches!(descriptor.signature, FunctionSignature::Exact(_)) {
                    return Ok((descriptor.id, order));
                }
                fallback = Some((descriptor.id, order));
            }
        }
        if let Some(resolved) = fallback {
            return Ok(resolved);
        }
        if let Some(name) = duplicate {
            return Err(CoreError::DuplicateArgument(name));
        }
        if let Some(name) = missing {
            return Err(CoreError::MissingArgument(name));
        }
        Err(CoreError::NoMatchingFunction {
            name: name.into(),
            arguments: positional_types
                .iter()
                .chain(named_types.iter().map(|(_, ty)| ty))
                .map(|type_id| type_id.map_or("structural".into(), |id| self.type_name(id).into()))
                .collect(),
        })
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
        argument_types: &[TypeId],
    ) -> Result<FunctionId, CoreError> {
        let candidates = self
            .functions_by_name
            .get(name)
            .ok_or_else(|| CoreError::UnknownFunction(name.to_string()))?;

        for id in candidates {
            let function = &self.functions[id.index];
            if matches!(
                &function.signature,
                FunctionSignature::Exact(types) if types.as_slice() == argument_types
            ) {
                return Ok(*id);
            }
        }

        let mut default_match = None;
        for id in candidates {
            let function = &self.functions[id.index];
            let FunctionSignature::Exact(types) = &function.signature else {
                continue;
            };
            if argument_types.len() >= types.len()
                || types[..argument_types.len()] != *argument_types
                || function.parameter_defaults[argument_types.len()..]
                    .iter()
                    .any(Option::is_none)
            {
                continue;
            }
            if default_match.is_none_or(|(_, missing): (FunctionId, usize)| {
                types.len() - argument_types.len() < missing
            }) {
                default_match = Some((*id, types.len() - argument_types.len()));
            }
        }
        if let Some((id, _)) = default_match {
            return Ok(id);
        }

        if argument_types.len() == 1
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
            arguments: argument_types
                .iter()
                .map(|id| self.type_name(*id).to_string())
                .collect(),
        })
    }

    /// Resolves the generic one-value fallback without inventing a scalar type.
    ///
    /// Containers have no [`TypeId`], but a function such as `print` can still
    /// explicitly accept one complete value through [`FunctionSignature::AnySingle`].
    /// Keeping this path separate prevents an array from being mistaken for an
    /// element scalar when exact overloads are registered alongside the fallback.
    pub fn resolve_any_single_function(&self, name: &str) -> Result<FunctionId, CoreError> {
        let candidates = self
            .functions_by_name
            .get(name)
            .ok_or_else(|| CoreError::UnknownFunction(name.to_string()))?;
        candidates
            .iter()
            .find(|id| {
                matches!(
                    self.functions[id.index].signature,
                    FunctionSignature::AnySingle
                )
            })
            .copied()
            .ok_or_else(|| CoreError::NoMatchingFunction {
                name: name.to_string(),
                arguments: vec!["array".to_string()],
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

#[cfg(test)]
#[path = "functions.tests.rs"]
mod tests;
