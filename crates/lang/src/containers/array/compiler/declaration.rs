//! Declared and inferred array types, array literals, and their shared element type.

use crate::ir::{TypedExpression, TypedExpressionKind};
use crate::semantic::{ArrayElementMode, ArrayType, SemanticType, ValueType};
use crate::syntax::{Expression, TypeExpression};

use crate::CompileError;

use crate::compiler::Compiler;

impl Compiler<'_> {
    /// Resolves one parsed array element type into its semantic contract.
    ///
    /// The base name is resolved through the registry, so any registered type
    /// may own an array. A `<subtype>` element constraint is resolved by literal
    /// suffix first and semantic subtype name second, matching the two spellings
    /// `register_subtype` accepts. The adaptive mode is set only when the
    /// element base is spelled with the `int` alias, which distinguishes `int[]`
    /// from an explicit `int64[]` even though both name the same base type.
    pub(crate) fn resolve_array_type(
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
    pub(crate) fn compile_array_expression(
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
}
