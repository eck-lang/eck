//! Compilation of array element contracts, literals, and element access.
//!
//! An array's static type is its element contract: the declared or inferred
//! element base type, an optional constrained element subtype, and its element
//! representation mode. Every element is compiled against that contract
//! here, and the index conversion the runtime needs is resolved once so element
//! access never repeats registry work during execution.

use crate::semantic::{
    ArrayElementMode, ArrayType, BinaryOperator as CoreBinaryOperator, IndexExtractor,
    ResolvedSubtypeConversion, SemanticType, TypeId, Value, ValueType,
};
use crate::syntax::{Expression, TypeExpression};
use std::{collections::HashMap, sync::Arc};

use crate::CompileError;
use crate::ir::{
    ArrayMethod, BindingId, CompleteTypeDomain, TypedExpression, TypedExpressionKind,
    TypedIndexDispatch,
};

use super::Compiler;

/// Every source spelling of a built-in array end operation.
///
/// This table binds source spellings to operations in one place, which makes
/// `append` exactly the operation of `push` and `prepend` exactly the
/// operation of `unshift`. The compiler emits the canonical operation for an
/// alias, so the runtime needs no second implementation, and diagnostics list
/// the supported spellings from the same table.
const ARRAY_METHODS: &[(&str, ArrayMethod)] = &[
    ("push", ArrayMethod::Push),
    ("append", ArrayMethod::Push),
    ("pop", ArrayMethod::Pop),
    ("unshift", ArrayMethod::Unshift),
    ("prepend", ArrayMethod::Unshift),
    ("shift", ArrayMethod::Shift),
];

/// Resolves one source method name to its built-in array end operation.
///
/// Returns `None` for a name no array operation claims, which lets the caller
/// report the supported spellings instead of a generic unknown-function error.
fn array_method(method_name: &str) -> Option<ArrayMethod> {
    ARRAY_METHODS
        .iter()
        .find(|(spelling, _)| *spelling == method_name)
        .map(|(_, method)| *method)
}

/// Returns the supported array method spellings for a diagnostic.
fn supported_array_method_names() -> String {
    ARRAY_METHODS
        .iter()
        .map(|(spelling, _)| *spelling)
        .collect::<Vec<_>>()
        .join(", ")
}

impl Compiler<'_> {
    /// Resolves one parsed array element type into its semantic contract.
    ///
    /// The base name is resolved through the registry, so any registered type
    /// may own an array. A `<subtype>` element constraint is resolved by literal
    /// suffix first and semantic subtype name second, matching the two spellings
    /// `register_subtype` accepts. The adaptive mode is set only when the
    /// element base is spelled with the `int` alias, which distinguishes `int[]`
    /// from an explicit `int64[]` even though both name the same base type.
    pub(super) fn resolve_array_type(
        &self,
        element_type: &TypeExpression,
        span: crate::syntax::Span,
    ) -> Result<ArrayType, CompileError> {
        let (base_text, subtype_text) = match element_type {
            TypeExpression::Named { name, .. } => (name.as_str(), None),
            TypeExpression::Qualified { base, subtype, .. } => {
                let TypeExpression::Named { name, .. } = base.as_ref() else {
                    return Err(CompileError::new(
                        span,
                        "array element type must have a named base",
                    ));
                };
                (name.as_str(), Some(subtype.as_str()))
            }
            TypeExpression::Array { .. } => {
                return Err(CompileError::new(span, "nested arrays are not supported"));
            }
            TypeExpression::Nullable { .. } => {
                return Err(CompileError::new(
                    span,
                    "nullable array elements are not supported",
                ));
            }
        };
        let base = self.registry.type_by_name(base_text).ok_or_else(|| {
            CompileError::new(span, format!("unknown array element type `{base_text}`"))
        })?;
        let subtype = match subtype_text {
            Some(subtype_text) => Some(
                self.registry
                    .subtype_by_suffix(subtype_text)
                    .or_else(|| self.registry.subtype_by_name(subtype_text))
                    .ok_or_else(|| {
                        CompileError::new(
                            span,
                            format!("unknown array element subtype `{subtype_text}`"),
                        )
                    })?,
            ),
            None => None,
        };
        Ok(ArrayType {
            element: ValueType { base, subtype },
            element_mode: if base_text == "int" {
                ArrayElementMode::AdaptiveInt
            } else {
                ArrayElementMode::Exact
            },
        })
    }

    /// Compiles one array literal against a declared or inferred element contract.
    ///
    /// A declared contract constrains every element through
    /// [`Compiler::compile_array_element`]. Without a declaration the element
    /// type is inferred from the compiled elements: an identical complete type
    /// is kept, a shared base with differing subtypes becomes an unconstrained
    /// base, and incompatible bases are rejected. An empty literal without a
    /// declaration cannot infer a type and is rejected.
    pub(super) fn compile_array_expression(
        &mut self,
        expression: &Expression,
        declared: Option<ArrayType>,
    ) -> Result<TypedExpression, CompileError> {
        let Expression::ArrayLiteral { elements, span } = expression else {
            return Err(CompileError::new(
                expression.span(),
                "an array binding must be initialized with an array literal",
            ));
        };
        let mut typed_elements = Vec::with_capacity(elements.len());
        let array_type = match declared {
            Some(array_type) => {
                for element in elements {
                    typed_elements.push(self.compile_array_element(element, array_type)?);
                }
                array_type
            }
            None => {
                if elements.is_empty() {
                    return Err(CompileError::new(
                        *span,
                        "cannot infer the element type of an empty array; add an element type annotation such as `int[]`",
                    ));
                }
                let mut element_types = Vec::with_capacity(elements.len());
                for element in elements {
                    let typed = match self.compile_expression(element, None) {
                        Ok(typed) => typed,
                        Err(error) => {
                            let Some(default_integer) = self.registry.default_integer().ok() else {
                                return Err(error);
                            };
                            if !Self::is_integer_literal_range_error(&error) {
                                return Err(error);
                            }
                            match self.compile_widened_integer_literal(element, default_integer)? {
                                Some(typed) => typed,
                                None => return Err(error),
                            }
                        }
                    };
                    self.validate_element_for_storage(&typed)?;
                    if typed.array_type().is_some() {
                        return Err(CompileError::new(
                            element.span(),
                            "nested arrays are not supported",
                        ));
                    }
                    let element_type = match typed.output {
                        Some(SemanticType::Scalar(element_type)) => element_type,
                        Some(SemanticType::Array(_)) => {
                            unreachable!("array elements are rejected before scalar matching")
                        }
                        None => {
                            return Err(CompileError::new(
                                element.span(),
                                "an array element must produce a value",
                            ));
                        }
                    };
                    element_types.push(element_type);
                    typed_elements.push(typed);
                }
                let adaptive_literals = elements
                    .iter()
                    .zip(&typed_elements)
                    .all(|(source, typed)| self.is_adaptive_integer_element_source(source, typed));
                let element =
                    self.common_array_element_type(&element_types, *span, adaptive_literals)?;
                let element_mode = if adaptive_literals {
                    ArrayElementMode::AdaptiveInt
                } else {
                    ArrayElementMode::Exact
                };
                ArrayType {
                    element,
                    element_mode,
                }
            }
        };
        // An element reaches storage only through the array it initializes, so
        // the destination contract is applied here for both a declared and an
        // inferred element type.
        let elements = typed_elements
            .into_iter()
            .map(|element| self.prepare_element_for_storage(element, array_type))
            .collect::<Result<Vec<_>, CompileError>>()?;
        Ok(TypedExpression {
            output: Some(SemanticType::Array(array_type)),
            kind: TypedExpressionKind::ArrayLiteral { elements },
            span: *span,
        })
    }

    /// Applies one array element representation contract to a compiled element.
    ///
    /// Every value that enters array storage must satisfy the element contract
    /// the array declared, but the value only exists after its expression has
    /// been evaluated, and evaluating an integer expression may temporarily
    /// promote it to a wider representation. The destination contract therefore
    /// has to survive until the value crosses into storage, which this resolves
    /// by wrapping the element in [`TypedExpressionKind::ElementStore`] whenever
    /// the runtime has to enforce it.
    ///
    /// Three destinations need no runtime work, and their element crosses into
    /// storage unchanged. An adaptive `int` element accepts the representation
    /// its expression produced, an element base without a fixed integer
    /// representation has no width to enforce, and a fixed-width element whose
    /// expression already proves to carry the declared representation is not
    /// checked again.
    pub(super) fn prepare_element_for_storage(
        &self,
        element: TypedExpression,
        array_type: ArrayType,
    ) -> Result<TypedExpression, CompileError> {
        self.validate_element_for_storage(&element)?;
        let span = element.span;
        if array_type.element_mode == ArrayElementMode::AdaptiveInt
            || !self
                .registry
                .is_integer_type(array_type.element.base)
                .map_err(|error| CompileError::core(span, error))?
            || Self::element_representation_is_exact(&element)
        {
            return Ok(element);
        }
        Ok(TypedExpression {
            output: element.output,
            kind: TypedExpressionKind::ElementStore {
                element: array_type.element,
                expression: Box::new(element),
            },
            span,
        })
    }

    /// Rejects a value that cannot cross into non-nullable array storage.
    ///
    /// This is shared by every storage boundary. It also runs before a
    /// constrained subtype conversion is built, because wrapping a nullable
    /// variable in a conversion would otherwise hide its nullability from the
    /// later preparation step.
    pub(super) fn validate_element_for_storage(
        &self,
        element: &TypedExpression,
    ) -> Result<(), CompileError> {
        self.require_non_nullable_expression(element)?;
        let null_type = self.registry.default_null().ok();
        if matches!(
            element.output,
            Some(SemanticType::Scalar(value_type))
                if Some(value_type.base) == null_type
        ) {
            return Err(CompileError::new(
                element.span,
                "nullable value must be narrowed before this operation",
            ));
        }
        Ok(())
    }

    /// Reports whether an element already carries the declared representation.
    ///
    /// Only the shapes the compiler can prove leave a fixed-width store free of
    /// runtime work. A literal is exactly the value it names, a conversion that
    /// names a base type always produces that base, and a read of a fixed-width
    /// array element carries the representation that array's own stores enforced.
    /// Every other shape may have promoted to a wider integer while it was
    /// evaluated, so its value is checked when it crosses into storage.
    fn element_representation_is_exact(element: &TypedExpression) -> bool {
        match &element.kind {
            TypedExpressionKind::Literal(_) => true,
            TypedExpressionKind::Convert {
                target_base: Some(_),
                ..
            } => true,
            TypedExpressionKind::ElementAccess { array, .. } => array
                .array_type()
                .is_some_and(|array_type| array_type.element_mode == ArrayElementMode::Exact),
            _ => false,
        }
    }

    /// Computes the element type shared by every element of an inferred literal.
    ///
    /// Identical complete types are preserved. When the bases match but the
    /// subtypes differ, the common base type is inferred with the subtype left
    /// unconstrained, because the container cannot pick one element's subtype
    /// over the others. Differing bases cannot form an array.
    fn common_array_element_type(
        &self,
        element_types: &[ValueType],
        span: crate::syntax::Span,
        adaptive_literals: bool,
    ) -> Result<ValueType, CompileError> {
        let first = element_types[0];
        if adaptive_literals
            && element_types
                .iter()
                .all(|element_type| self.is_signed_integer_base(element_type.base))
        {
            let subtype = element_types
                .iter()
                .all(|element_type| element_type.subtype == first.subtype)
                .then_some(first.subtype)
                .flatten();
            let base = self
                .registry
                .default_integer()
                .map_err(|error| CompileError::core(span, error))?;
            return Ok(ValueType { base, subtype });
        }
        if element_types
            .iter()
            .all(|element_type| *element_type == first)
        {
            return Ok(first);
        }
        if element_types
            .iter()
            .all(|element_type| element_type.base == first.base)
        {
            return Ok(ValueType::plain(first.base));
        }
        let differing = element_types
            .iter()
            .find(|element_type| element_type.base != first.base)
            .copied()
            .expect("a differing element type was found above");
        Err(CompileError::new(
            span,
            format!(
                "array elements must share a base type; found `{}` and `{}`",
                self.registry.value_type_name(first),
                self.registry.value_type_name(differing)
            ),
        ))
    }

    /// Compiles one element against an array's declared element contract.
    ///
    /// An unconstrained element keeps the value's own subtype. An adaptive
    /// integer array also accepts a wider integer value, and a literal that
    /// exceeds the declared width is retried with a wider integer type. A
    /// constrained element is converted to the declared subtype with a
    /// compile-time-resolved scale and cast back to the declared base, which is
    /// why `int<mm>[] = [2cm]` stores `20mm`.
    pub(super) fn compile_array_element(
        &mut self,
        expression: &Expression,
        array_type: ArrayType,
    ) -> Result<TypedExpression, CompileError> {
        let declared = array_type.element;
        let typed = match self.compile_expression(expression, Some(declared.base)) {
            Ok(typed) => typed,
            Err(error) => {
                let widened = array_type.element_mode == ArrayElementMode::AdaptiveInt
                    && Self::is_integer_literal_expression(expression)
                    && Self::is_integer_literal_range_error(&error);
                match widened {
                    true => {
                        match self.compile_widened_integer_literal(expression, declared.base)? {
                            Some(typed) => typed,
                            None => return Err(error),
                        }
                    }
                    false => return Err(error),
                }
            }
        };
        self.validate_element_for_storage(&typed)?;
        if typed.array_type().is_some() {
            return Err(CompileError::new(
                expression.span(),
                "nested arrays are not supported",
            ));
        }
        let actual = match typed.output {
            Some(SemanticType::Scalar(actual)) => actual,
            Some(SemanticType::Array(_)) => {
                unreachable!("array elements are rejected before scalar matching")
            }
            None => {
                return Err(CompileError::new(
                    expression.span(),
                    "an array element must produce a value",
                ));
            }
        };
        let complete_type_domain = typed.complete_type_domain();
        let adaptive_domain_is_valid = complete_type_domain.as_ref().is_none_or(|domain| {
            domain
                .candidates
                .iter()
                .all(|candidate| self.is_signed_integer_base(candidate.base))
        });
        if array_type.element_mode == ArrayElementMode::AdaptiveInt
            && (!adaptive_domain_is_valid
                || complete_type_domain.is_none() && !self.is_signed_integer_base(actual.base))
        {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "type mismatch: array element expects `{}`, expression produces `{}`",
                    self.registry.value_type_name(declared),
                    self.registry.value_type_name(actual)
                ),
            ));
        }
        if let Some(target_subtype) = declared.subtype {
            if let Some(source_domain) = complete_type_domain.clone() {
                return self.dynamic_convert(
                    typed,
                    source_domain,
                    None,
                    Some(target_subtype),
                    format!(
                        "array element conversion to `{}`",
                        self.registry.value_type_name(declared)
                    ),
                    expression.span(),
                );
            }
            if actual == declared {
                return Ok(typed);
            }
            let conversion = self
                .registry
                .resolve_subtype_conversion(actual, target_subtype)
                .map_err(|error| CompileError::core(expression.span(), error))?;
            let target_base = if array_type.element_mode == ArrayElementMode::AdaptiveInt
                && conversion.output.base != declared.base
                && self.is_signed_integer_base(conversion.output.base)
            {
                None
            } else {
                Some(declared.base)
            };
            let output_base = target_base.unwrap_or(conversion.output.base);
            let output = ValueType::qualified(output_base, target_subtype);
            if let TypedExpressionKind::Literal(value) = &typed.kind
                && self.literal_conversion_succeeds(
                    value,
                    conversion.scale,
                    output,
                    expression.span(),
                )? == Some(false)
            {
                return Err(CompileError::new(
                    expression.span(),
                    format!(
                        "array element `{}` cannot be represented as `{}`",
                        self.registry.value_type_name(value.value_type()),
                        self.registry.value_type_name(output)
                    ),
                ));
            }
            let (scale_plan, _) =
                self.compile_scale_plan(actual.base, conversion.scale, expression.span())?;
            return Ok(TypedExpression {
                output: Some(SemanticType::Scalar(output)),
                kind: TypedExpressionKind::Convert {
                    conversion: ResolvedSubtypeConversion {
                        output,
                        scale: conversion.scale,
                    },
                    target_base,
                    scale_plan,
                    expression: Box::new(typed),
                },
                span: expression.span(),
            });
        }
        if complete_type_domain.is_some() {
            let integer_destination = self
                .registry
                .is_integer_type(declared.base)
                .map_err(|error| CompileError::core(expression.span(), error))?;
            let compatible = complete_type_domain.as_ref().is_some_and(|domain| {
                domain.candidates.iter().all(|candidate| {
                    candidate.base == declared.base
                        || (integer_destination
                            && self
                                .registry
                                .is_integer_type(candidate.base)
                                .unwrap_or(false))
                })
            });
            if compatible {
                return Ok(typed);
            }
        }
        if actual.base == declared.base {
            return Ok(typed);
        }
        if array_type.element_mode == ArrayElementMode::AdaptiveInt
            && self.is_signed_integer_base(actual.base)
        {
            return Ok(typed);
        }
        Err(CompileError::new(
            expression.span(),
            format!(
                "type mismatch: array element expects `{}`, expression produces `{}`",
                self.registry.value_type_name(declared),
                self.registry.value_type_name(actual)
            ),
        ))
    }

    /// Reports whether one expression is a parser-shaped signed integer literal.
    fn is_integer_literal_expression(expression: &Expression) -> bool {
        match expression {
            Expression::Number { raw_text, .. } => !raw_text.contains(['.', 'e', 'E']),
            Expression::Unary {
                operator: crate::syntax::UnaryOperator::Negation,
                operand,
                ..
            } => {
                matches!(operand.as_ref(), Expression::Number { raw_text, .. } if !raw_text.contains(['.', 'e', 'E']))
            }
            _ => false,
        }
    }

    /// Reports whether a literal error is the width failure adaptive widening may repair.
    fn is_integer_literal_range_error(error: &CompileError) -> bool {
        error.message.starts_with("invalid literal ")
            && (error
                .message
                .contains("number too large to fit in target type")
                || error
                    .message
                    .contains("number too small to fit in target type"))
    }

    /// Reports whether one inferred element retains adaptive `int` semantics.
    ///
    /// Literal elements are adaptive by default. A scalar variable contributes
    /// that mode only when its declaration spelled `int` or omitted the scalar
    /// annotation; an explicit `int64` source remains a fixed-width contract.
    fn is_adaptive_integer_element_source(
        &self,
        source: &Expression,
        typed: &TypedExpression,
    ) -> bool {
        let Some(SemanticType::Scalar(value_type)) = typed.output else {
            return false;
        };
        if !self.is_signed_integer_base(value_type.base) {
            return false;
        }
        if Self::is_integer_literal_expression(source) {
            return true;
        }
        let Expression::Variable { name, .. } = source else {
            return false;
        };
        self.resolve_variable(name)
            .is_some_and(|variable| variable.adaptive_integer)
    }

    /// Builds the complete identities an array element may carry at runtime.
    ///
    /// Adaptive `int[]` values may use every signed widening representation,
    /// while an exact array keeps its declared base. A constrained contract
    /// fixes the subtype; an unconstrained contract admits the plain identity
    /// and every registered subtype for the permitted base family.
    pub(super) fn array_element_complete_type_domain(
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

    /// Re-parses one integer literal with the next signed integer type.
    ///
    /// Adaptive `int` elements accept integer values that exceed the declared
    /// base width. A literal that the declared base cannot represent is retried
    /// against the registry's explicit signed widening progression, and the
    /// first representation that accepts it is used. Returns `None` when no
    /// wider signed type accepts the literal, leaving the original diagnostic.
    fn compile_widened_integer_literal(
        &self,
        expression: &Expression,
        declared_base: TypeId,
    ) -> Result<Option<TypedExpression>, CompileError> {
        let Some((raw_text, suffix, span)) = Self::integer_literal_parts(expression) else {
            return Ok(None);
        };
        let widening_types = self.registry.signed_integer_widening_types();
        let Some(declared_index) = widening_types
            .iter()
            .position(|candidate| *candidate == declared_base)
        else {
            return Ok(None);
        };
        let subtype = suffix
            .map(|suffix| {
                self.registry.subtype_by_suffix(suffix).ok_or_else(|| {
                    CompileError::new(span, format!("unknown numeric literal suffix `{suffix}`"))
                })
            })
            .transpose()?;
        for candidate in widening_types.into_iter().skip(declared_index + 1) {
            let Ok(mut value) = self.registry.parse_numeric(&raw_text, Some(candidate)) else {
                continue;
            };
            if let Some(subtype) = subtype {
                value = value.with_subtype(Some(subtype));
            }
            return Ok(Some(TypedExpression {
                output: Some(SemanticType::Scalar(value.value_type())),
                kind: TypedExpressionKind::Literal(value),
                span,
            }));
        }
        Ok(None)
    }

    /// Returns the signed source text, suffix, and span of an integer literal.
    fn integer_literal_parts(
        expression: &Expression,
    ) -> Option<(String, Option<&str>, crate::syntax::Span)> {
        match expression {
            Expression::Number {
                raw_text,
                suffix,
                span,
            } if !raw_text.contains(['.', 'e', 'E']) => {
                Some((raw_text.clone(), suffix.as_deref(), *span))
            }
            Expression::Unary {
                operator: crate::syntax::UnaryOperator::Negation,
                operand,
                span,
            } => match operand.as_ref() {
                Expression::Number {
                    raw_text, suffix, ..
                } if !raw_text.contains(['.', 'e', 'E']) => {
                    Some((format!("-{raw_text}"), suffix.as_deref(), *span))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Compiles one array index and resolves its runtime conversion once.
    ///
    /// The index must be a plain integer whose type registered an index
    /// extractor. A literal index is converted at compile time so constant
    /// access carries no runtime conversion work; a dynamic index keeps the
    /// resolved extractor in the typed program instead of looking it up during
    /// execution.
    pub(super) fn compile_array_index(
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

    /// Compiles one built-in end operation on an array receiver.
    ///
    /// The receiver must be a directly named mutable array binding, because the
    /// operation mutates that binding and a mutation of a temporary array could
    /// never be observed.
    ///
    /// A value to insert is compiled against the array's declared element
    /// contract through [`Compiler::compile_array_element`] and then crossed
    /// into storage through [`Compiler::prepare_element_for_storage`], so
    /// `push` and `unshift` enforce exactly the representation rules of an
    /// array literal and of an indexed assignment. Insertion therefore has no
    /// conversion or overflow path of its own: a fixed-width array rejects a
    /// value its representation cannot hold, and an adaptive `int` array
    /// accepts the wider representation the value already has.
    ///
    /// Every end operation invalidates the recorded static element types of the
    /// receiver, because an insertion stores a value whose runtime
    /// representation the compiler cannot guarantee and a removal repositions
    /// every remaining element. Later reads of the binding then dispatch on the
    /// subtype each stored value actually carries.
    pub(super) fn compile_array_method(
        &mut self,
        receiver: &Expression,
        typed_receiver: &TypedExpression,
        array_type: ArrayType,
        method_name: &str,
        arguments: &[Expression],
        span: crate::syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let method = array_method(method_name).ok_or_else(|| {
            CompileError::new(
                span,
                format!(
                    "an array has no method `{method_name}`; the supported methods are {}",
                    supported_array_method_names()
                ),
            )
        })?;
        let Expression::Variable { name, .. } = receiver else {
            return Err(CompileError::new(
                receiver.span(),
                format!(
                    "`{method_name}` mutates an array, so its receiver must be an array binding"
                ),
            ));
        };
        let variable = self.resolve_variable(name).ok_or_else(|| {
            CompileError::new(receiver.span(), format!("unknown binding `{name}`"))
        })?;
        if !variable.mutable {
            return Err(CompileError::new(
                receiver.span(),
                format!("cannot call `{method_name}` through immutable binding `{name}`"),
            ));
        }
        let (binding, slot) = match &typed_receiver.kind {
            TypedExpressionKind::Variable { binding, slot, .. } => (*binding, *slot),
            _ => unreachable!("an array receiver is always compiled as a variable"),
        };
        let stored_value = match method {
            ArrayMethod::Push | ArrayMethod::Unshift => {
                let [value] = arguments else {
                    return Err(CompileError::new(
                        span,
                        format!("`{method_name}` expects exactly one value to add"),
                    ));
                };
                let element = self.compile_array_element(value, array_type)?;
                Some(self.prepare_element_for_storage(element, array_type)?)
            }
            ArrayMethod::Pop | ArrayMethod::Shift => {
                if !arguments.is_empty() {
                    return Err(CompileError::new(
                        span,
                        format!("`{method_name}` removes one element and takes no arguments"),
                    ));
                }
                None
            }
        };
        // A removal produces the declared element type. An unconstrained
        // element keeps whatever subtype it was stored with, so the produced
        // complete type is only known at runtime in that case. An insertion
        // produces no value at all.
        let (output, result_domain, empty_result) = match method.removes_element() {
            // The null value an empty removal produces is resolved here, so the
            // runtime never looks the null type up or re-parses its literal.
            true => (
                Some(SemanticType::Scalar(array_type.element)),
                (array_type.element_mode == ArrayElementMode::AdaptiveInt
                    || array_type.element.subtype.is_none())
                .then(|| self.array_element_complete_type_domain(array_type)),
                Some(
                    self.registry
                        .parse_null("null", None)
                        .map_err(|error| CompileError::core(span, error))?,
                ),
            ),
            false => (None, None, None),
        };
        self.array_element_types.remove(&binding);
        Ok(TypedExpression {
            output,
            kind: TypedExpressionKind::ArrayMethod {
                method,
                binding,
                slot,
                arguments: stored_value.into_iter().collect(),
                result_domain,
                empty_result,
            },
            span,
        })
    }

    /// Records the element slots of a freshly bound array.
    ///
    /// A literal initializer contributes each element's own type, which keeps an
    /// unconstrained array's per-element subtypes available to constant reads.
    /// An element whose type is only known at runtime records a dynamic slot, so
    /// a later read of it still dispatches on the subtype the value holds. An
    /// array copied from another binding inherits that binding's record so the
    /// alias behaves like its source, and any other initializer clears it.
    pub(super) fn record_array_element_types(
        &mut self,
        binding: BindingId,
        expression: &TypedExpression,
    ) {
        match &expression.kind {
            TypedExpressionKind::ArrayLiteral { elements, .. } => {
                let element_types: Vec<Option<ValueType>> = elements
                    .iter()
                    .map(|element| {
                        if element.complete_type_domain().is_some() {
                            None
                        } else {
                            match element.output {
                                Some(SemanticType::Scalar(element_type)) => Some(element_type),
                                Some(SemanticType::Array(_)) | None => None,
                            }
                        }
                    })
                    .collect();
                self.array_element_types.insert(binding, element_types);
            }
            TypedExpressionKind::Variable {
                binding: source, ..
            } if matches!(expression.output, Some(SemanticType::Array(_))) => {
                match self.array_element_types.get(source).cloned() {
                    Some(element_types) => {
                        self.array_element_types.insert(binding, element_types);
                    }
                    None => {
                        self.array_element_types.remove(&binding);
                    }
                }
            }
            _ => {
                self.array_element_types.remove(&binding);
            }
        }
    }

    /// Returns the recorded element slot for one constant element read.
    ///
    /// The outer option reports whether the array still has a record at all: a
    /// miss means the whole array is dynamic. A present inner `None` means the
    /// slot itself is dynamic, while `Some` carries the element's static type.
    pub(super) fn array_element_type(
        &self,
        array: &TypedExpression,
        constant_index: Option<usize>,
    ) -> Option<Option<ValueType>> {
        let TypedExpressionKind::Variable { binding, .. } = &array.kind else {
            return None;
        };
        let index = constant_index?;
        self.array_element_types
            .get(binding)?
            .get(index)
            .copied()
            .or(Some(None))
    }

    /// Updates one recorded element slot after a constant element write.
    ///
    /// `None` records a dynamic slot, because the written value's subtype is
    /// only known at runtime. An index outside the recorded range, or a binding
    /// without a record, leaves the record as it is; the read then falls back to
    /// dispatching on the stored subtype.
    pub(super) fn update_array_element_type(
        &mut self,
        binding: BindingId,
        index: usize,
        element_slot: Option<ValueType>,
    ) {
        if let Some(element_types) = self.array_element_types.get_mut(&binding)
            && index < element_types.len()
        {
            element_types[index] = element_slot;
        }
    }

    /// Captures the known element types before compiling a control-flow path.
    pub(super) fn array_element_type_snapshot(&self) -> HashMap<BindingId, Vec<Option<ValueType>>> {
        self.array_element_types.clone()
    }

    /// Restores the known element types before compiling another control-flow path.
    pub(super) fn restore_array_element_type_snapshot(
        &mut self,
        snapshot: HashMap<BindingId, Vec<Option<ValueType>>>,
    ) {
        self.array_element_types = snapshot;
    }

    /// Keeps an element type only when every possible control-flow path agrees.
    ///
    /// A branch or loop may leave an array unchanged, so retaining the type
    /// observed while compiling only its body would make a static operation use
    /// the wrong subtype at runtime. Disagreement marks just that element
    /// dynamic and reuses the existing subtype dispatch path.
    pub(super) fn merge_array_element_type_snapshots(
        &mut self,
        snapshots: &[HashMap<BindingId, Vec<Option<ValueType>>>],
    ) {
        self.array_element_types = Self::join_array_element_type_snapshots(snapshots);
    }

    /// Joins element type snapshots into the state every path agrees on.
    ///
    /// The join is the analysis' meet operation on one abstract flow state. A
    /// binding survives only when every snapshot records it with the same
    /// length, and an element keeps its complete type only when every snapshot
    /// records that exact type. Every other element becomes dynamic, so a later
    /// read dispatches on the subtype the stored value actually holds.
    ///
    /// Returning the joined state instead of assigning it lets a loop compare
    /// the head of one iteration with the head of the next while looking for a
    /// fixed point.
    pub(super) fn join_array_element_type_snapshots(
        snapshots: &[HashMap<BindingId, Vec<Option<ValueType>>>],
    ) -> HashMap<BindingId, Vec<Option<ValueType>>> {
        let Some(first) = snapshots.first() else {
            return HashMap::new();
        };
        let mut merged = HashMap::new();
        for (binding, elements) in first {
            let recorded = snapshots
                .iter()
                .filter_map(|snapshot| snapshot.get(binding))
                .collect::<Vec<_>>();
            if recorded.len() != snapshots.len()
                || recorded.iter().any(|other| other.len() != elements.len())
            {
                continue;
            }
            let joined = elements
                .iter()
                .enumerate()
                .map(|(index, element)| {
                    let agrees = recorded.iter().all(|other| other[index] == *element);
                    match agrees {
                        true => *element,
                        false => None,
                    }
                })
                .collect();
            merged.insert(*binding, joined);
        }
        merged
    }

    /// Reports whether one literal element converts exactly to the declared type.
    ///
    /// The steps replay the runtime conversion with the same registered scale
    /// operators and the same base cast, so an element the declared element type
    /// cannot represent is rejected while compiling instead of failing later.
    /// `None` reports that a scale step needs execution services, which only the
    /// runtime can apply, so the conversion stays a runtime step.
    fn literal_conversion_succeeds(
        &self,
        value: &Value,
        scale: crate::semantic::Scale,
        declared: ValueType,
        span: crate::syntax::Span,
    ) -> Result<Option<bool>, CompileError> {
        let mut scaled = value.clone().with_subtype(None);
        if scale.numerator != 1 {
            let factor = self
                .registry
                .parse_numeric(&scale.numerator.to_string(), Some(scaled.type_id()))
                .map_err(|error| CompileError::core(span, error))?;
            let Some(descriptor) = self.context_free_operator(
                CoreBinaryOperator::Multiplication,
                scaled.type_id(),
                factor.type_id(),
                span,
            )?
            else {
                return Ok(None);
            };
            scaled = (descriptor.execute)(&scaled, &factor)
                .map_err(|error| CompileError::core(span, error))?;
        }
        if scale.denominator != 1 {
            let divisor_type = if self.registry.default_integer().ok() == Some(scaled.type_id()) {
                self.registry
                    .default_fractional()
                    .map_err(|error| CompileError::core(span, error))?
            } else {
                scaled.type_id()
            };
            let factor = self
                .registry
                .parse_numeric(&scale.denominator.to_string(), Some(divisor_type))
                .map_err(|error| CompileError::core(span, error))?;
            if self.integer_division_is_exact(&scaled, &factor, declared.base, span)? == Some(false)
            {
                return Ok(Some(false));
            }
            let Some(descriptor) = self.context_free_operator(
                CoreBinaryOperator::Division,
                scaled.type_id(),
                factor.type_id(),
                span,
            )?
            else {
                return Ok(None);
            };
            scaled = (descriptor.execute)(&scaled, &factor)
                .map_err(|error| CompileError::core(span, error))?;
        }
        if scaled.type_id() == declared.base {
            return Ok(Some(true));
        }
        // Mirror the runtime base cast: format with the type's own formatter and
        // re-parse the text with the declared base type.
        let formatter = self
            .registry
            .type_descriptor(scaled.type_id())
            .map_err(|error| CompileError::core(span, error))?
            .format;
        let formatted = formatter(&scaled).map_err(|error| CompileError::core(span, error))?;
        if self
            .registry
            .parse_numeric(&formatted, Some(declared.base))
            .is_ok()
        {
            return Ok(Some(true));
        }
        if let Some((whole, fractional)) = formatted.split_once('.')
            && fractional.trim_end_matches('0').is_empty()
        {
            let whole = if whole.is_empty() || whole == "-" || whole == "+" {
                format!("{whole}0")
            } else {
                whole.to_string()
            };
            if self
                .registry
                .parse_numeric(&whole, Some(declared.base))
                .is_ok()
            {
                return Ok(Some(true));
            }
        }
        Ok(Some(false))
    }

    /// Checks exactness before a fixed integer conversion divides an integer.
    ///
    /// The ordinary integer division operator intentionally truncates. A
    /// subtype conversion cannot use that result as evidence of
    /// representability, so it asks the registered remainder operation first.
    /// `None` is reserved for a non-integer scale or a target whose division is
    /// already fractional; those cases keep the normal scalar conversion path.
    fn integer_division_is_exact(
        &self,
        dividend: &Value,
        divisor: &Value,
        target_base: TypeId,
        span: crate::syntax::Span,
    ) -> Result<Option<bool>, CompileError> {
        if !self
            .registry
            .is_integer_type(target_base)
            .map_err(|error| CompileError::core(span, error))?
            || !self
                .registry
                .is_integer_type(dividend.type_id())
                .map_err(|error| CompileError::core(span, error))?
            || !self
                .registry
                .is_integer_type(divisor.type_id())
                .map_err(|error| CompileError::core(span, error))?
        {
            return Ok(None);
        }
        let operator = self
            .registry
            .resolve_binary_operator(
                CoreBinaryOperator::Remainder,
                dividend.type_id(),
                divisor.type_id(),
            )
            .map_err(|error| CompileError::core(span, error))?;
        let descriptor = self
            .registry
            .operator(operator)
            .map_err(|error| CompileError::core(span, error))?;
        let remainder = (descriptor.execute)(dividend, divisor)
            .map_err(|error| CompileError::core(span, error))?;
        let formatted = self
            .registry
            .format_value(&remainder)
            .map_err(|error| CompileError::core(span, error))?;
        Ok(Some(formatted == "0"))
    }

    /// Resolves one scale operator that does not need execution services.
    ///
    /// Returns `None` when the operator has a value-dependent implementation, so
    /// the caller keeps the step for the runtime instead of mis-folding it.
    fn context_free_operator(
        &self,
        operator: CoreBinaryOperator,
        left_operand: TypeId,
        right_operand: TypeId,
        span: crate::syntax::Span,
    ) -> Result<Option<&crate::semantic::BinaryOperatorDescriptor>, CompileError> {
        let id = self
            .registry
            .resolve_binary_operator(operator, left_operand, right_operand)
            .map_err(|error| CompileError::core(span, error))?;
        let descriptor = self
            .registry
            .operator(id)
            .map_err(|error| CompileError::core(span, error))?;
        if descriptor.context_execute.is_some() {
            return Ok(None);
        }
        Ok(Some(descriptor))
    }
}

#[cfg(test)]
#[path = "arrays.tests.rs"]
mod tests;
