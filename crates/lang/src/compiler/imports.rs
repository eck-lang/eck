//! Import resolution for `use` declarations.
//!
//! Resolves namespace aliases, explicit member imports, and wildcard imports
//! into the lexical import scope each source block introduces. The registry
//! owns the namespaces; this module only tracks which local names are bound.

use crate::CompileError;
use crate::semantic::TypeId;
use crate::syntax::{SourceIdentifier, UseClause, UseDeclaration};

use super::{Compiler, FunctionImport, FunctionImportSource, NamespaceImport};

impl Compiler<'_> {
    /// Applies one `use` declaration to the active lexical import scope.
    pub(super) fn compile_use_declaration(
        &mut self,
        declaration: &UseDeclaration,
    ) -> Result<(), CompileError> {
        match &declaration.clause {
            UseClause::Namespace { namespace, alias } => {
                self.require_namespace(namespace)?;
                let local_name = alias.as_ref().unwrap_or(namespace);
                self.insert_namespace_import(local_name, &namespace.name)
            }
            UseClause::Members { namespace, members } => {
                self.require_namespace(namespace)?;
                for imported_member in members {
                    self.registry
                        .namespace_symbol(&namespace.name, &imported_member.name.name)
                        .map_err(|error| CompileError::core(imported_member.name.span, error))?;
                    let local_name = imported_member
                        .alias
                        .as_ref()
                        .unwrap_or(&imported_member.name);
                    self.insert_explicit_function_import(
                        local_name,
                        &namespace.name,
                        &imported_member.name.name,
                    )?;
                }
                Ok(())
            }
            UseClause::Wildcard { namespace, alias } => {
                self.require_namespace(namespace)?;
                if let Some(alias) = alias {
                    return self.insert_namespace_import(alias, &namespace.name);
                }
                for member in self
                    .registry
                    .namespace_member_names(&namespace.name)
                    .map_err(|error| CompileError::core(namespace.span, error))?
                {
                    self.insert_wildcard_function_import(member, &namespace.name, namespace.span)?;
                }
                Ok(())
            }
        }
    }

    /// Validates that an imported namespace exists in the semantic registry.
    fn require_namespace(&self, namespace: &SourceIdentifier) -> Result<(), CompileError> {
        if self.registry.has_namespace(&namespace.name) {
            Ok(())
        } else {
            Err(CompileError::new(
                namespace.span,
                format!("unknown namespace `{}`", namespace.name),
            ))
        }
    }

    /// Inserts one namespace alias while rejecting same-scope import collisions.
    fn insert_namespace_import(
        &mut self,
        local_name: &SourceIdentifier,
        namespace: &str,
    ) -> Result<(), CompileError> {
        let scope = self
            .import_scopes
            .last_mut()
            .expect("compiler always has an import scope");
        if scope.namespaces.contains_key(&local_name.name)
            || scope.functions.contains_key(&local_name.name)
        {
            return Err(CompileError::new(
                local_name.span,
                format!(
                    "import name `{}` is already defined in this scope",
                    local_name.name
                ),
            ));
        }
        scope.namespaces.insert(
            local_name.name.clone(),
            NamespaceImport {
                namespace: namespace.to_string(),
            },
        );
        Ok(())
    }

    /// Inserts a selective member import, for which every collision is an error.
    fn insert_explicit_function_import(
        &mut self,
        local_name: &SourceIdentifier,
        namespace: &str,
        member: &str,
    ) -> Result<(), CompileError> {
        let scope = self
            .import_scopes
            .last_mut()
            .expect("compiler always has an import scope");
        if scope.namespaces.contains_key(&local_name.name)
            || scope.functions.contains_key(&local_name.name)
        {
            return Err(CompileError::new(
                local_name.span,
                format!(
                    "import name `{}` is already defined in this scope",
                    local_name.name
                ),
            ));
        }
        scope.functions.insert(
            local_name.name.clone(),
            FunctionImport::Unique(FunctionImportSource {
                namespace: namespace.to_string(),
                member: member.to_string(),
                wildcard: false,
            }),
        );
        Ok(())
    }

    /// Adds one wildcard member and retains competing wildcard sources as ambiguity.
    fn insert_wildcard_function_import(
        &mut self,
        member: &str,
        namespace: &str,
        span: crate::syntax::Span,
    ) -> Result<(), CompileError> {
        let scope = self
            .import_scopes
            .last_mut()
            .expect("compiler always has an import scope");
        if scope.namespaces.contains_key(member) {
            return Err(CompileError::new(
                span,
                format!("import name `{member}` is already defined in this scope"),
            ));
        }
        let source = FunctionImportSource {
            namespace: namespace.to_string(),
            member: member.to_string(),
            wildcard: true,
        };
        match scope.functions.entry(member.to_string()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(FunctionImport::Unique(source));
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => match entry.get_mut() {
                FunctionImport::Unique(existing)
                    if existing.wildcard && existing.namespace != namespace =>
                {
                    let existing = existing.clone();
                    entry.insert(FunctionImport::Ambiguous(vec![existing, source]));
                }
                FunctionImport::Ambiguous(sources)
                    if !sources.iter().any(|source| source.namespace == namespace) =>
                {
                    sources.push(source);
                }
                _ => {
                    return Err(CompileError::new(
                        span,
                        format!("import name `{member}` is already defined in this scope"),
                    ));
                }
            },
        }
        Ok(())
    }

    /// Finds the nearest active namespace import for a local name.
    fn resolve_namespace_import(&self, local_name: &str) -> Option<&NamespaceImport> {
        self.import_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.namespaces.get(local_name))
    }

    /// Finds the nearest active function import for a local name.
    fn resolve_function_import(&self, local_name: &str) -> Option<&FunctionImport> {
        self.import_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.functions.get(local_name))
    }

    /// Resolves a source call through a namespace alias, lexical import, or global.
    pub(super) fn resolve_source_function(
        &self,
        namespace: Option<&SourceIdentifier>,
        function: &SourceIdentifier,
        argument_types: &[TypeId],
        call_span: crate::syntax::Span,
    ) -> Result<crate::semantic::FunctionId, CompileError> {
        if let Some(namespace) = namespace {
            let namespace_import =
                self.resolve_namespace_import(&namespace.name)
                    .ok_or_else(|| {
                        CompileError::new(
                            namespace.span,
                            format!("namespace `{}` is not imported", namespace.name),
                        )
                    })?;
            return self
                .registry
                .resolve_namespace_function(
                    &namespace_import.namespace,
                    &function.name,
                    argument_types,
                )
                .map_err(|error| CompileError::core(function.span, error));
        }

        if let Some(import) = self.resolve_function_import(&function.name) {
            return match import {
                FunctionImport::Unique(source) => self
                    .registry
                    .resolve_namespace_function(&source.namespace, &source.member, argument_types)
                    .map_err(|error| CompileError::core(call_span, error)),
                FunctionImport::Ambiguous(sources) => {
                    let mut candidates: Vec<_> = sources
                        .iter()
                        .map(|source| format!("{}.{}", source.namespace, source.member))
                        .collect();
                    candidates.sort();
                    candidates.dedup();
                    Err(CompileError::new(
                        function.span,
                        format!(
                            "imported function `{}` is ambiguous; possible sources: {}",
                            function.name,
                            candidates.join(", ")
                        ),
                    ))
                }
            };
        }

        if self.registry.is_global_function(&function.name) {
            return self
                .registry
                .resolve_function(&function.name, argument_types)
                .map_err(|error| CompileError::core(call_span, error));
        }

        Err(CompileError::new(
            function.span,
            format!(
                "function `{}` is not in scope; import it with `use` or call it through an imported namespace",
                function.name
            ),
        ))
    }
}
