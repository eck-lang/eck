//! The element contract every value crosses into array storage through.

use crate::ir::{TypedExpression, TypedExpressionKind};
use crate::semantic::{
    ArrayType, BinaryOperator as CoreBinaryOperator, ResolvedSubtypeConversion,
    ScalarRepresentation, SemanticType, TypeId, Value, ValueType,
};
use crate::syntax::Expression;

use crate::CompileError;

use crate::compiler::Compiler;

impl Compiler<'_> {
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
    pub(crate) fn prepare_element_for_storage(
        &self,
        element: TypedExpression,
        array_type: ArrayType,
    ) -> Result<TypedExpression, CompileError> {
        let span = element.span;
        let Some(SemanticType::Scalar(declared)) = array_type.static_semantic_type() else {
            return Ok(element);
        };
        self.validate_element_for_storage(&element)?;
        if array_type.static_representation() == Some(ScalarRepresentation::AdaptiveSignedInteger)
            || !self
                .registry
                .is_integer_type(declared.base)
                .map_err(|error| CompileError::core(span, error))?
            || Self::element_representation_is_exact(&element)
        {
            return Ok(element);
        }
        Ok(TypedExpression {
            output: element.output.clone(),
            kind: TypedExpressionKind::ElementStore {
                element: *declared,
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
    pub(crate) fn validate_element_for_storage(
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

    /// Applies structural membership plus the array's scalar representation policy.
    ///
    /// The ordinary semantic predicate handles exact union membership and
    /// invariant recursive arrays. An adaptive `int` member is the one case
    /// where a concrete scalar may legitimately widen beyond the member's
    /// default `TypeId`; that policy is local to this array boundary and never
    /// creates a runtime union identity.
    fn array_element_is_assignable(
        &self,
        source: &SemanticType,
        destination: &SemanticType,
        representation: ScalarRepresentation,
    ) -> bool {
        if crate::semantic::is_assignable(source, destination) {
            return true;
        }
        if representation != ScalarRepresentation::AdaptiveSignedInteger {
            return false;
        }
        match (source, destination) {
            (SemanticType::Union(source_members), _) => source_members.iter().all(|member| {
                self.array_element_is_assignable(member, destination, representation)
            }),
            (_, SemanticType::Union(destination_members)) => destination_members
                .iter()
                .any(|member| self.array_element_is_assignable(source, member, representation)),
            (SemanticType::Scalar(source), SemanticType::Scalar(destination)) => {
                self.registry.default_integer().ok() == Some(destination.base)
                    && self.is_signed_integer_base(source.base)
                    && (destination.subtype.is_none() || source.subtype == destination.subtype)
            }
            _ => false,
        }
    }

    /// Reports whether an element already carries the declared representation.
    ///
    /// Only the shapes the compiler can prove leave a fixed-width store free of
    /// runtime work. A literal is exactly the value it names, a conversion that
    /// names a base type always produces that base, and a read of a fixed-width
    /// array element carries the representation that array's own stores enforced.
    /// Every other shape may have promoted to a wider integer while it was
    /// evaluated, so its value is checked when it crosses into storage.
    ///
    /// This is the compile-time half of one rule whose runtime half is
    /// [`crate::containers::array::element_crosses_unchanged`]: the compiler emits a store
    /// without an element check exactly when this proof holds.
    fn element_representation_is_exact(element: &TypedExpression) -> bool {
        match &element.kind {
            TypedExpressionKind::Literal(_) => true,
            TypedExpressionKind::Convert {
                target_base: Some(_),
                ..
            } => true,
            TypedExpressionKind::ElementAccess { array, .. } => {
                array.array_type().is_some_and(|array_type| {
                    array_type.static_representation() == Some(ScalarRepresentation::Exact)
                })
            }
            _ => false,
        }
    }

    /// Compiles one element against an array's declared element contract.
    ///
    /// An unconstrained element keeps the value's own subtype. An adaptive
    /// integer array also accepts a wider integer value, and a literal that
    /// exceeds the declared width is retried with a wider integer type. A
    /// constrained element is converted to the declared subtype with a
    /// compile-time-resolved scale and cast back to the declared base, which is
    /// why `int<mm>[] = [2cm]` stores `20mm`.
    pub(crate) fn compile_array_element(
        &mut self,
        expression: &Expression,
        array_type: ArrayType,
    ) -> Result<TypedExpression, CompileError> {
        let Some(static_element) = array_type.static_semantic_type() else {
            return match self.compile_expression(expression, None) {
                Ok(typed) => Ok(typed),
                Err(error) if Self::is_integer_literal_range_error(&error) => {
                    let default_integer = self
                        .registry
                        .default_integer()
                        .map_err(|core| CompileError::core(expression.span(), core))?;
                    self.compile_widened_integer_literal(expression, default_integer)?
                        .ok_or(error)
                }
                Err(error) => Err(error),
            };
        };
        let SemanticType::Scalar(declared) = static_element else {
            let typed = match static_element {
                SemanticType::Array(inner)
                    if matches!(expression, Expression::ArrayLiteral { .. }) =>
                {
                    self.compile_array_expression(expression, Some((**inner).clone()))?
                }
                _ => match self.compile_expression(expression, None) {
                    Ok(typed) => typed,
                    Err(error)
                        if array_type.static_representation()
                            == Some(ScalarRepresentation::AdaptiveSignedInteger)
                            && Self::is_integer_literal_expression(expression)
                            && Self::is_integer_literal_range_error(&error) =>
                    {
                        let Some(default_integer) = self.registry.default_integer().ok() else {
                            return Err(error);
                        };
                        match self.compile_widened_integer_literal(expression, default_integer)? {
                            Some(typed) => typed,
                            None => return Err(error),
                        }
                    }
                    Err(error) => return Err(error),
                },
            };
            let actual = typed.output.as_ref().ok_or_else(|| {
                CompileError::new(expression.span(), "an array element must produce a value")
            })?;
            if !self.array_element_is_assignable(
                actual,
                static_element,
                array_type
                    .static_representation()
                    .expect("a static element has a representation"),
            ) {
                return Err(CompileError::new(
                    expression.span(),
                    "array element does not satisfy its recursive array contract",
                ));
            }
            return Ok(typed);
        };
        let declared = *declared;
        let typed = match self.compile_expression(expression, Some(declared.base)) {
            Ok(typed) => typed,
            Err(error) => {
                let widened = array_type.static_representation()
                    == Some(ScalarRepresentation::AdaptiveSignedInteger)
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
        let actual = match typed.output {
            Some(SemanticType::Scalar(actual)) => actual,
            Some(SemanticType::Open)
            | Some(SemanticType::Array(_))
            | Some(SemanticType::Map(_))
            | Some(SemanticType::Union(_)) => {
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
        if array_type.static_representation() == Some(ScalarRepresentation::AdaptiveSignedInteger)
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
            let target_base = if array_type.static_representation()
                == Some(ScalarRepresentation::AdaptiveSignedInteger)
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
        if array_type.static_representation() == Some(ScalarRepresentation::AdaptiveSignedInteger)
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
    pub(super) fn is_integer_literal_expression(expression: &Expression) -> bool {
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
    pub(super) fn is_integer_literal_range_error(error: &CompileError) -> bool {
        error.message.starts_with("invalid literal ")
            && (error
                .message
                .contains("number too large to fit in target type")
                || error
                    .message
                    .contains("number too small to fit in target type"))
    }

    /// Re-parses one integer literal with the next signed integer type.
    ///
    /// Adaptive `int` elements accept integer values that exceed the declared
    /// base width. A literal that the declared base cannot represent is retried
    /// against the registry's explicit signed widening progression, and the
    /// first representation that accepts it is used. Returns `None` when no
    /// wider signed type accepts the literal, leaving the original diagnostic.
    pub(super) fn compile_widened_integer_literal(
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

    /// Multiplies one literal magnitude by the numerator of a scale.
    ///
    /// The ordinary integer operators promote their result on overflow, so the
    /// representation a product has is not decided by the operand types alone and
    /// [`Compiler::context_free_operator`] refuses them. A literal is a fixed
    /// magnitude, so the product is computed through the widest registered
    /// integer instead: the promotion the runtime applies preserves the
    /// magnitude, so both paths agree on the value the destination must hold.
    ///
    /// `None` reports that no registered type expresses the product, which leaves
    /// the conversion a runtime step.
    fn scaled_literal_product(
        &self,
        value: &Value,
        factor: &Value,
        span: crate::syntax::Span,
    ) -> Result<Option<Value>, CompileError> {
        if let Some(descriptor) = self.context_free_operator(
            CoreBinaryOperator::Multiplication,
            value.type_id(),
            factor.type_id(),
            span,
        )? {
            return (descriptor.execute)(value, factor)
                .map(Some)
                .map_err(|error| CompileError::core(span, error));
        }
        let integer_operands = self
            .registry
            .is_integer_type(value.type_id())
            .unwrap_or(false)
            && self
                .registry
                .is_integer_type(factor.type_id())
                .unwrap_or(false);
        if !integer_operands {
            return Ok(None);
        }
        let Some(widest) = self
            .registry
            .signed_integer_widening_types()
            .last()
            .copied()
        else {
            return Ok(None);
        };
        let wide_value = self.widen_magnitude(value, widest, span)?;
        let wide_factor = self.widen_magnitude(factor, widest, span)?;
        let Some(descriptor) =
            self.context_free_operator(CoreBinaryOperator::Multiplication, widest, widest, span)?
        else {
            return Ok(None);
        };
        (descriptor.execute)(&wide_value, &wide_factor)
            .map(Some)
            .map_err(|error| CompileError::core(span, error))
    }

    /// Converts one literal magnitude into the representation used to fold it.
    ///
    /// The conversion round-trips through the source formatter and the target
    /// parser, so a magnitude the folding representation cannot hold is reported
    /// as a compile error instead of folding a truncated value.
    fn widen_magnitude(
        &self,
        value: &Value,
        target: TypeId,
        span: crate::syntax::Span,
    ) -> Result<Value, CompileError> {
        self.registry
            .convert_base(value, target)
            .map_err(|error| CompileError::core(span, error))
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
            match self.scaled_literal_product(&scaled, &factor, span)? {
                Some(product) => scaled = product,
                None => return Ok(None),
            }
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
