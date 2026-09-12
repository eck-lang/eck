//! Namespace registration and namespace-aware native function resolution.

use std::collections::HashMap;

use crate::{CoreError, FunctionId, NamespaceSymbol, TypeId};

use super::Registry;

/// Stores one namespace's exported surface inside the semantic registry.
pub(super) struct RegisteredNamespace {
    pub(super) symbols: HashMap<&'static str, NamespaceSymbol>,
}

impl Registry {
    /// Registers an empty namespace and optionally associates it with a receiver type.
    ///
    /// A receiver association lets pipe expressions resolve a member through the
    /// same namespace surface used by qualified and imported calls.
    pub fn register_namespace(
        &mut self,
        name: &'static str,
        receiver_type: Option<TypeId>,
    ) -> Result<(), CoreError> {
        if self.namespaces.contains_key(name) {
            return Err(CoreError::DuplicateNamespace(name.to_string()));
        }
        if let Some(type_id) = receiver_type {
            self.type_descriptor(type_id)?;
            if self.namespaces_by_receiver_type.contains_key(&type_id) {
                return Err(CoreError::DuplicateTypeNamespace(
                    self.type_name(type_id).to_string(),
                ));
            }
            self.namespaces_by_receiver_type.insert(type_id, name);
        }
        self.namespaces.insert(
            name,
            RegisteredNamespace {
                symbols: HashMap::new(),
            },
        );
        Ok(())
    }

    /// Exports a registered native function family as one namespace member.
    pub fn export_namespace_function(
        &mut self,
        namespace: &str,
        member: &'static str,
        function_name: &'static str,
    ) -> Result<(), CoreError> {
        if !self.functions_by_name.contains_key(function_name) {
            return Err(CoreError::UnknownFunction(function_name.to_string()));
        }
        let registered_namespace = self
            .namespaces
            .get_mut(namespace)
            .ok_or_else(|| CoreError::UnknownNamespace(namespace.to_string()))?;
        if registered_namespace.symbols.contains_key(member) {
            return Err(CoreError::DuplicateNamespaceMember {
                namespace: namespace.to_string(),
                member: member.to_string(),
            });
        }
        registered_namespace
            .symbols
            .insert(member, NamespaceSymbol::Function { function_name });
        Ok(())
    }

    /// Returns whether a namespace with the given public name is registered.
    pub fn has_namespace(&self, name: &str) -> bool {
        self.namespaces.contains_key(name)
    }

    /// Returns the exported member names of a namespace in deterministic order.
    pub fn namespace_member_names(&self, namespace: &str) -> Result<Vec<&str>, CoreError> {
        let registered_namespace = self
            .namespaces
            .get(namespace)
            .ok_or_else(|| CoreError::UnknownNamespace(namespace.to_string()))?;
        let mut names: Vec<_> = registered_namespace.symbols.keys().copied().collect();
        names.sort_unstable();
        Ok(names)
    }

    /// Returns one namespace symbol without performing function overload resolution.
    pub fn namespace_symbol(
        &self,
        namespace: &str,
        member: &str,
    ) -> Result<&NamespaceSymbol, CoreError> {
        self.namespaces
            .get(namespace)
            .ok_or_else(|| CoreError::UnknownNamespace(namespace.to_string()))?
            .symbols
            .get(member)
            .ok_or_else(|| CoreError::UnknownNamespaceMember {
                namespace: namespace.to_string(),
                member: member.to_string(),
            })
    }

    /// Resolves a namespaced function member for exact argument base types.
    pub fn resolve_namespace_function(
        &self,
        namespace: &str,
        member: &str,
        argument_types: &[TypeId],
    ) -> Result<FunctionId, CoreError> {
        match self.namespace_symbol(namespace, member)? {
            NamespaceSymbol::Function { function_name } => {
                self.resolve_function(function_name, argument_types)
            }
        }
    }

    /// Resolves a pipe member through the namespace associated with its receiver type.
    pub fn resolve_receiver_function(
        &self,
        receiver_type: TypeId,
        member: &str,
        argument_types: &[TypeId],
    ) -> Result<FunctionId, CoreError> {
        let namespace = self
            .namespaces_by_receiver_type
            .get(&receiver_type)
            .ok_or_else(|| CoreError::UnknownFunction(member.to_string()))?;
        self.resolve_namespace_function(namespace, member, argument_types)
    }
}

#[cfg(test)]
#[path = "namespaces.tests.rs"]
mod tests;
