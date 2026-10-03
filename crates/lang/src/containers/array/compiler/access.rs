//! Element access: index compilation and the recorded element facts.

use crate::ir::{CompleteTypeDomain, TypedExpression, TypedExpressionKind, TypedIndexDispatch};
use crate::semantic::{ArrayType, IndexExtractor, ScalarRepresentation, SemanticType, ValueType};
use crate::syntax::Expression;
use std::sync::Arc;

use crate::CompileError;

use crate::compiler::Compiler;

impl Compiler<'_> {
    /// Describes a finite observed element set without creating an array contract.
    pub(crate) fn finite_array_element_output(
        &self,
        types: impl IntoIterator<Item = SemanticType>,
    ) -> Option<(SemanticType, Option<Arc<CompleteTypeDomain>>)> {
        let types = types.into_iter().collect::<Vec<_>>();
        if types.is_empty() {
            return None;
        }
        let semantic_type = SemanticType::union(types);
        let scalar_candidates = match &semantic_type {
            SemanticType::Scalar(value_type) => vec![*value_type],
            SemanticType::Union(members)
                if members
                    .iter()
                    .all(|member| matches!(member, SemanticType::Scalar(_))) =>
            {
                members.iter().filter_map(SemanticType::as_scalar).collect()
            }
            SemanticType::Open
            | SemanticType::Array(_)
            | SemanticType::Map(_)
            | SemanticType::Source(_)
            | SemanticType::Row(_)
            | SemanticType::Union(_) => {
                return Some((semantic_type, None));
            }
        };
        if scalar_candidates.len() <= 1 {
            return Some((semantic_type, None));
        }
        Some((
            SemanticType::Scalar(scalar_candidates[0]),
            Some(CompleteTypeDomain::from_candidates(
                self.registry,
                scalar_candidates,
            )),
        ))
    }

    /// Returns every concrete array contract a static array union may carry.
    /// A nullable array is intentionally excluded until its null branch has
    /// been narrowed; a union of arrays remains homogeneous per runtime value.
    pub(crate) fn array_types_for_access(
        &self,
        expression: &TypedExpression,
    ) -> Option<Vec<ArrayType>> {
        match expression.output.as_ref()? {
            SemanticType::Array(array_type) => Some(vec![(**array_type).clone()]),
            SemanticType::Union(members) => {
                let array_types = members
                    .iter()
                    .map(|member| match member {
                        SemanticType::Array(array_type) => Some((**array_type).clone()),
                        SemanticType::Open
                        | SemanticType::Scalar(_)
                        | SemanticType::Map(_)
                        | SemanticType::Source(_)
                        | SemanticType::Row(_)
                        | SemanticType::Union(_) => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                (!array_types.is_empty()).then_some(array_types)
            }
            SemanticType::Open
            | SemanticType::Scalar(_)
            | SemanticType::Map(_)
            | SemanticType::Source(_)
            | SemanticType::Row(_) => None,
        }
    }

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
            Some(SemanticType::Open)
            | Some(SemanticType::Array(_))
            | Some(SemanticType::Map(_))
            | Some(SemanticType::Source(_))
            | Some(SemanticType::Row(_))
            | Some(SemanticType::Union(_)) => {
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

    /// Preserves adaptive width dispatch while honoring proven plain element subtypes.
    ///
    /// Typed adaptive arrays can normally contain arbitrary registered subtypes.
    /// A finite flow domain containing only plain signed integers proves that
    /// qualified alternatives cannot occur at this program point. Every signed
    /// width remains available so overflow promotion keeps its ordinary plans.
    pub(crate) fn array_element_access_complete_type_domain(
        &self,
        array: &TypedExpression,
        array_type: ArrayType,
    ) -> Arc<CompleteTypeDomain> {
        let plain_signed = array_type.static_representation() == Some(ScalarRepresentation::AdaptiveSignedInteger)
            && self.array_known_element_types(array).is_some_and(|known| {
                !known.is_empty() && known.iter().all(|semantic_type| matches!(semantic_type,
                    SemanticType::Scalar(value_type) if value_type.subtype.is_none() && self.is_signed_integer_base(value_type.base)))
            });
        let domain = self.array_element_complete_type_domain(array_type);
        if plain_signed {
            CompleteTypeDomain::from_candidates(
                self.registry,
                domain
                    .candidates
                    .iter()
                    .copied()
                    .filter(|candidate| candidate.subtype.is_none()),
            )
        } else {
            domain
        }
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
        let Some(SemanticType::Scalar(element)) = array_type.static_semantic_type() else {
            return CompleteTypeDomain::from_candidates(self.registry, []);
        };
        let bases = if array_type.static_representation()
            == Some(ScalarRepresentation::AdaptiveSignedInteger)
        {
            self.registry.signed_integer_widening_types()
        } else {
            vec![element.base]
        };
        let subtypes = match element.subtype {
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

#[cfg(test)]
#[path = "access.tests.rs"]
mod tests;
