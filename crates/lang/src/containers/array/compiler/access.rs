//! Element access: index compilation and the recorded element facts.

use crate::ir::{CompleteTypeDomain, TypedExpression, TypedExpressionKind, TypedIndexDispatch};
use crate::semantic::{ArrayElementMode, ArrayType, IndexExtractor, SemanticType, ValueType};
use crate::syntax::Expression;
use std::sync::Arc;

use crate::CompileError;

use crate::compiler::Compiler;

impl Compiler<'_> {
    /// Compiles one array index and resolves its runtime conversion once.
    ///
    /// The index must be a plain integer whose type registered an index
    /// extractor. A literal index is converted at compile time so constant
    /// access carries no runtime conversion work; a dynamic index keeps the
    /// resolved extractor in the typed program instead of looking it up during
    /// execution.
    pub(crate) fn compile_array_index(
        &mut self,
        index: &Expression,
    ) -> Result<
        (
            TypedExpression,
            Option<usize>,
            IndexExtractor,
            Option<TypedIndexDispatch>,
        ),
        CompileError,
    > {
        let typed = self.compile_expression(index, None)?;
        self.require_scalar_expression(&typed)?;
        let index_type = match typed.output {
            Some(SemanticType::Scalar(index_type)) => index_type,
            Some(SemanticType::Array(_)) => {
                unreachable!("require_scalar_expression rejects array semantic types")
            }
            None => {
                return Err(CompileError::new(
                    index.span(),
                    "an array index must produce a value",
                ));
            }
        };
        let is_plain_integer = index_type.subtype.is_none()
            && self
                .registry
                .is_integer_type(index_type.base)
                .map_err(|error| CompileError::core(index.span(), error))?;
        if !is_plain_integer {
            return Err(CompileError::new(
                index.span(),
                format!(
                    "an array index must be a plain integer, found `{}`",
                    self.registry.value_type_name(index_type)
                ),
            ));
        }
        let index_extractor = self
            .registry
            .index_extractor(index_type.base)
            .ok_or_else(|| {
                CompileError::new(
                    index.span(),
                    format!(
                        "type `{}` cannot be used as an array index",
                        self.registry.type_name(index_type.base)
                    ),
                )
            })?;
        let index_dispatch = typed
            .complete_type_domain()
            .map(|domain| {
                for candidate in domain.candidates.iter().copied() {
                    if !self
                        .registry
                        .is_integer_type(candidate.base)
                        .unwrap_or(false)
                    {
                        return Err(CompileError::new(
                            index.span(),
                            format!(
                                "an array index must be a plain integer, found `{}`",
                                self.registry.value_type_name(candidate)
                            ),
                        ));
                    }
                    if self.registry.index_extractor(candidate.base).is_none() {
                        return Err(CompileError::new(
                            index.span(),
                            format!(
                                "type `{}` cannot be used as an array index",
                                self.registry.type_name(candidate.base)
                            ),
                        ));
                    }
                }
                let extractors = domain
                    .candidates
                    .iter()
                    .map(|candidate| self.registry.index_extractor(candidate.base))
                    .collect();
                Ok(TypedIndexDispatch { domain, extractors })
            })
            .transpose()?;
        let constant_index = match &typed.kind {
            TypedExpressionKind::Literal(value) => index_extractor(value).ok(),
            _ => None,
        };
        Ok((typed, constant_index, index_extractor, index_dispatch))
    }

    /// Builds the complete identities an array element may carry at runtime.
    ///
    /// Adaptive `int[]` values may use every signed widening representation,
    /// while an exact array keeps its declared base. A constrained contract
    /// fixes the subtype; an unconstrained contract admits the plain identity
    /// and every registered subtype for the permitted base family.
    pub(crate) fn array_element_complete_type_domain(
        &self,
        array_type: ArrayType,
    ) -> Arc<CompleteTypeDomain> {
        let bases = if array_type.element_mode == ArrayElementMode::AdaptiveInt {
            self.registry.signed_integer_widening_types()
        } else {
            vec![array_type.element.base]
        };
        let subtypes = match array_type.element.subtype {
            Some(subtype) => vec![Some(subtype)],
            None => std::iter::once(None)
                .chain(self.registry.registered_subtype_ids().map(Some))
                .collect(),
        };
        CompleteTypeDomain::from_candidates(
            self.registry,
            bases.into_iter().flat_map(|base| {
                subtypes
                    .iter()
                    .copied()
                    .map(move |subtype| ValueType { base, subtype })
            }),
        )
    }
}
