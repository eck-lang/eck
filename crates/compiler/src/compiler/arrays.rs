//! Compilation of array element contracts, literals, and element access.
//!
//! An array's static type is its element contract: the declared or inferred
//! element base type, an optional constrained element subtype, and whether the
//! `int` element is adaptive. Every element is compiled against that contract
//! here, and the index conversion the runtime needs is resolved once so element
//! access never repeats registry work during execution.

use language_core::{
    BinaryOperator as CoreBinaryOperator, ComparisonOperator as CoreComparisonOperator, CoreError,
    IndexExtractor, ResolvedSubtypeConversion, Scale, SubtypeId, TypeId, Value, ValueType,
};
use std::collections::HashMap;
use syntax::Expression;

use crate::CompileError;
use ir::{ArrayMethod, ArrayType, BindingId, TypedExpression, TypedExpressionKind};

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
    /// Resolves one `base[]`, `base<subtype>[]` array type annotation.
    ///
    /// The base name is resolved through the registry, so any registered type
    /// may own an array. A `<subtype>` element constraint is resolved by literal
    /// suffix first and semantic subtype name second, matching the two spellings
    /// `register_subtype` accepts. The adaptive flag is set only when the
    /// element base is spelled with the `int` alias, which distinguishes `int[]`
    /// from an explicit `int64[]` even though both name the same base type.
    pub(super) fn resolve_array_type(
        &self,
        type_name: &str,
        span: syntax::Span,
    ) -> Result<ArrayType, CompileError> {
        let element_text = type_name.strip_suffix("[]").ok_or_else(|| {
            CompileError::new(span, format!("`{type_name}` is not an array type"))
        })?;
        let (base_text, subtype_text) = match element_text.split_once('<') {
            Some((base_text, remainder)) => {
                let subtype_text = remainder.strip_suffix('>').ok_or_else(|| {
                    CompileError::new(span, format!("invalid array element type `{element_text}`"))
                })?;
                (base_text, Some(subtype_text))
            }
            None => (element_text, None),
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
            adaptive_integer: base_text == "int",
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
                    let typed = self.compile_expression(element, None)?;
                    if typed.array_type().is_some() {
                        return Err(CompileError::new(
                            element.span(),
                            "nested arrays are not supported",
                        ));
                    }
                    let element_type = typed.output.ok_or_else(|| {
                        CompileError::new(element.span(), "an array element must produce a value")
                    })?;
                    element_types.push(element_type);
                    typed_elements.push(typed);
                }
                let element = self.common_array_element_type(&element_types, *span)?;
                let adaptive_integer = Some(element.base) == self.registry.default_integer().ok();
                ArrayType {
                    element,
                    adaptive_integer,
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
            output: Some(array_type.element),
            kind: TypedExpressionKind::ArrayLiteral {
                array_type,
                elements,
            },
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
        let span = element.span;
        if array_type.adaptive_integer
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
                .is_some_and(|array_type| !array_type.adaptive_integer),
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
        span: syntax::Span,
    ) -> Result<ValueType, CompileError> {
        let first = element_types[0];
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
                let widened = array_type.adaptive_integer
                    && declared.subtype.is_none()
                    && matches!(expression, Expression::Number { .. });
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
        if typed.array_type().is_some() {
            return Err(CompileError::new(
                expression.span(),
                "nested arrays are not supported",
            ));
        }
        let actual = typed.output.ok_or_else(|| {
            CompileError::new(expression.span(), "an array element must produce a value")
        })?;
        if let Some(target_subtype) = declared.subtype {
            if actual == declared {
                return Ok(typed);
            }
            let conversion = self
                .registry
                .resolve_subtype_conversion(actual, target_subtype)
                .map_err(|error| CompileError::core(expression.span(), error))?;
            let output = ValueType::qualified(declared.base, target_subtype);
            if let TypedExpressionKind::Literal(value) = &typed.kind
                && self.literal_conversion_succeeds(
                    value,
                    conversion.scale,
                    declared,
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
            return Ok(TypedExpression {
                output: Some(output),
                kind: TypedExpressionKind::Convert {
                    conversion: ResolvedSubtypeConversion {
                        output,
                        scale: conversion.scale,
                    },
                    target_base: Some(declared.base),
                    expression: Box::new(typed),
                },
                span: expression.span(),
            });
        }
        if actual.base == declared.base {
            return Ok(typed);
        }
        if array_type.adaptive_integer
            && self
                .registry
                .is_integer_type(actual.base)
                .map_err(|error| CompileError::core(expression.span(), error))?
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

    /// Re-parses one integer literal with a wider registered integer type.
    ///
    /// Adaptive `int` elements accept integer values that exceed the declared
    /// base width. A literal that the declared base cannot represent is retried
    /// against every other registered integer type in deterministic name order,
    /// and the first representation that accepts it is used. Returns `None` when
    /// no integer type accepts the literal, leaving the original diagnostic.
    fn compile_widened_integer_literal(
        &self,
        expression: &Expression,
        declared_base: TypeId,
    ) -> Result<Option<TypedExpression>, CompileError> {
        let Expression::Number {
            raw_text,
            suffix,
            span,
        } = expression
        else {
            return Ok(None);
        };
        for name in self.registry.registered_type_names() {
            let Some(candidate) = self.registry.type_by_name(name) else {
                continue;
            };
            if candidate == declared_base
                || !self
                    .registry
                    .is_integer_type(candidate)
                    .map_err(|error| CompileError::core(*span, error))?
            {
                continue;
            }
            let Ok(mut value) = self.registry.parse_numeric(raw_text, Some(candidate)) else {
                continue;
            };
            if let Some(suffix) = suffix {
                let subtype = self.registry.subtype_by_suffix(suffix).ok_or_else(|| {
                    CompileError::new(*span, format!("unknown numeric literal suffix `{suffix}`"))
                })?;
                value = value.with_subtype(Some(subtype));
            }
            return Ok(Some(TypedExpression {
                output: Some(value.value_type()),
                kind: TypedExpressionKind::Literal(value),
                span: *span,
            }));
        }
        Ok(None)
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
    ) -> Result<(TypedExpression, Option<usize>, IndexExtractor), CompileError> {
        let typed = self.compile_expression(index, None)?;
        self.require_scalar_expression(&typed)?;
        let index_type = typed.output.ok_or_else(|| {
            CompileError::new(index.span(), "an array index must produce a value")
        })?;
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
        let constant_index = match &typed.kind {
            TypedExpressionKind::Literal(value) => index_extractor(value).ok(),
            _ => None,
        };
        Ok((typed, constant_index, index_extractor))
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
        span: syntax::Span,
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
        let (output, dynamic_result, empty_result) = match method.removes_element() {
            // The null value an empty removal produces is resolved here, so the
            // runtime never looks the null type up or re-parses its literal.
            true => (
                Some(array_type.element),
                array_type.element.subtype.is_none(),
                Some(
                    self.registry
                        .parse_null("null", None)
                        .map_err(|error| CompileError::core(span, error))?,
                ),
            ),
            false => (None, false, None),
        };
        self.array_element_types.remove(&binding);
        Ok(TypedExpression {
            output,
            kind: TypedExpressionKind::ArrayMethod {
                method,
                binding,
                slot,
                arguments: stored_value.into_iter().collect(),
                dynamic_result,
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
                        if element.dynamic_complete_type() {
                            None
                        } else {
                            element.output
                        }
                    })
                    .collect();
                self.array_element_types.insert(binding, element_types);
            }
            TypedExpressionKind::Variable {
                binding: source,
                array_type: Some(_),
                ..
            } => match self.array_element_types.get(source).cloned() {
                Some(element_types) => {
                    self.array_element_types.insert(binding, element_types);
                }
                None => {
                    self.array_element_types.remove(&binding);
                }
            },
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

    /// Compiles one binary operation whose operand complete types are dynamic.
    ///
    /// An unconstrained array element keeps the subtype it was stored with, so
    /// the compiler cannot pick one operand type. It instead resolves the
    /// operation once per candidate subtype and stores the plans in a dispatch
    /// table. The runtime selects a plan from the value's own subtype with one
    /// slot computation, so no subtype rule is resolved during execution and no
    /// element subtype is silently erased.
    pub(super) fn dynamic_binary(
        &self,
        operator: CoreBinaryOperator,
        left_operand: TypedExpression,
        left_type: ValueType,
        right_operand: TypedExpression,
        right_type: ValueType,
        operands: DynamicOperands,
        span: syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let width = self.registry.subtype_dispatch_width();
        let (left_width, right_width) = operands.dispatch_widths(width);
        let left_slots = self.operand_dispatch_slots(left_width, left_type.subtype);
        let right_slots = self.operand_dispatch_slots(right_width, right_type.subtype);
        let mut plans = Vec::with_capacity(left_width * right_width);
        let mut first_error: Option<CompileError> = None;
        let mut reference_output: Option<ValueType> = None;
        let mut dynamic_result = false;
        for left_slot in &left_slots {
            for right_slot in &right_slots {
                let plan = match (left_slot, right_slot) {
                    (Some(left_candidate), Some(right_candidate)) => {
                        let (candidate_left, candidate_right) = operands.candidate_types(
                            left_type,
                            *left_candidate,
                            right_type,
                            *right_candidate,
                        );
                        match self.resolve_binary_plan(
                            operator,
                            candidate_left,
                            candidate_right,
                            span,
                        ) {
                            Ok(plan) => {
                                match reference_output {
                                    None => reference_output = Some(plan.resolution.output),
                                    Some(output) if output != plan.resolution.output => {
                                        // The result's own subtype varies with the
                                        // operand subtype, so enclosing operations must
                                        // keep dispatching on it.
                                        dynamic_result = true;
                                    }
                                    Some(_) => {}
                                }
                                Some(plan)
                            }
                            Err(error) => {
                                first_error.get_or_insert(error);
                                None
                            }
                        }
                    }
                    _ => None,
                };
                plans.push(plan);
            }
        }
        if plans.iter().all(|plan| plan.is_none()) {
            return Err(first_error.unwrap_or_else(|| {
                CompileError::new(
                    span,
                    format!("operator `{operator}` is not defined for the operand types"),
                )
            }));
        }
        let output = match reference_output {
            Some(output) if !dynamic_result => output,
            Some(output) => ValueType::plain(output.base),
            None => ValueType::plain(left_type.base),
        };
        Ok(TypedExpression {
            output: Some(output),
            kind: TypedExpressionKind::DynamicBinary {
                operator,
                dispatch: Box::new(ir::TypedBinaryDispatch {
                    left_width,
                    right_width,
                    plans,
                }),
                dynamic_result,
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
            },
            span,
        })
    }

    /// Compiles one comparison whose operand complete types are dynamic.
    ///
    /// The layout mirrors [`Compiler::dynamic_binary`]: the runtime selects the
    /// relation from the operand subtypes instead of resolving one comparison
    /// type at compile time.
    pub(super) fn dynamic_comparison(
        &self,
        operator: CoreComparisonOperator,
        left_operand: TypedExpression,
        left_type: ValueType,
        right_operand: TypedExpression,
        right_type: ValueType,
        operands: DynamicOperands,
        span: syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let width = self.registry.subtype_dispatch_width();
        let (left_width, right_width) = operands.dispatch_widths(width);
        let left_slots = self.operand_dispatch_slots(left_width, left_type.subtype);
        let right_slots = self.operand_dispatch_slots(right_width, right_type.subtype);
        let mut resolutions = Vec::with_capacity(left_width * right_width);
        let mut first_error: Option<CompileError> = None;
        let mut output: Option<ValueType> = None;
        for left_slot in &left_slots {
            for right_slot in &right_slots {
                let resolution = match (left_slot, right_slot) {
                    (Some(left_candidate), Some(right_candidate)) => {
                        let (candidate_left, candidate_right) = operands.candidate_types(
                            left_type,
                            *left_candidate,
                            right_type,
                            *right_candidate,
                        );
                        match self.registry.resolve_comparison_operation(
                            operator,
                            candidate_left,
                            candidate_right,
                        ) {
                            Ok(resolution) => {
                                output.get_or_insert(resolution.output);
                                Some(resolution)
                            }
                            Err(error) => {
                                first_error.get_or_insert(CompileError::core(span, error));
                                None
                            }
                        }
                    }
                    _ => None,
                };
                resolutions.push(resolution);
            }
        }
        if resolutions.iter().all(|resolution| resolution.is_none()) {
            return Err(first_error.unwrap_or_else(|| {
                CompileError::new(
                    span,
                    format!("comparison `{operator}` is not defined for the operand types"),
                )
            }));
        }
        Ok(TypedExpression {
            output,
            kind: TypedExpressionKind::DynamicComparison {
                operator,
                dispatch: Box::new(ir::TypedComparisonDispatch {
                    left_width,
                    right_width,
                    resolutions,
                }),
                left_operand: Box::new(left_operand),
                right_operand: Box::new(right_operand),
            },
            span,
        })
    }

    /// Returns the candidate subtype each dispatch slot of one operand addresses.
    ///
    /// A statically typed operand contributes a single slot holding its own
    /// subtype, so its slot index is always zero. A dynamically typed operand
    /// contributes one slot per registry dispatch slot: slot zero is
    /// unqualified and every registered subtype occupies the slot derived from
    /// its index. `None` marks a slot no runtime value can select.
    fn operand_dispatch_slots(
        &self,
        width: usize,
        static_subtype: Option<SubtypeId>,
    ) -> Vec<Option<Option<SubtypeId>>> {
        if width == 1 {
            return vec![Some(static_subtype)];
        }
        let mut slots = vec![None; width];
        slots[0] = Some(None);
        for subtype in self.registry.registered_subtype_ids() {
            slots[self.registry.subtype_dispatch_slot(Some(subtype))] = Some(Some(subtype));
        }
        slots
    }

    /// Resolves one candidate complete operand pair into an executable plan.
    ///
    /// The resolution follows the same order the static path uses: relative
    /// addition and subtraction rules first, then the ordinary operator, then
    /// the operand scale plans and the optional relative-adjustment operator.
    /// The resulting plan is exactly what the runtime replays, so no subtype
    /// rule is resolved during execution.
    fn resolve_binary_plan(
        &self,
        operator: CoreBinaryOperator,
        left_type: ValueType,
        right_type: ValueType,
        span: syntax::Span,
    ) -> Result<ir::TypedBinaryPlan, CompileError> {
        let resolution = match operator {
            CoreBinaryOperator::Addition | CoreBinaryOperator::Subtraction => {
                match self
                    .registry
                    .resolve_subtype_relative_rule(operator, left_type, right_type)
                {
                    Ok(relative) => relative,
                    Err(CoreError::SubtypeRelativeOperatorNotDefined { .. }) => self
                        .registry
                        .resolve_binary_operation(operator, left_type, right_type)
                        .map_err(|error| CompileError::core(span, error))?,
                    Err(error) => return Err(CompileError::core(span, error)),
                }
            }
            _ => self
                .registry
                .resolve_binary_operation(operator, left_type, right_type)
                .map_err(|error| CompileError::core(span, error))?,
        };
        let (left_operand_scale, scaled_left_type) =
            self.compile_scale_plan(left_type.base, resolution.left_operand_scale, span)?;
        let right_scale = resolution
            .relative_adjustment
            .unwrap_or(resolution.right_operand_scale);
        let (right_operand_scale, scaled_right_type) =
            self.compile_scale_plan(right_type.base, right_scale, span)?;
        let relative_adjustment_operator = if resolution.relative_adjustment.is_some() {
            Some(
                self.registry
                    .resolve_binary_operator(
                        CoreBinaryOperator::Multiplication,
                        scaled_left_type,
                        scaled_right_type,
                    )
                    .map_err(|error| CompileError::core(span, error))?,
            )
        } else {
            None
        };
        Ok(ir::TypedBinaryPlan {
            resolution,
            execution_plan: ir::TypedBinaryExecutionPlan {
                left_operand_scale,
                right_operand_scale,
                relative_adjustment_operator,
            },
        })
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
        scale: language_core::Scale,
        declared: ValueType,
        span: syntax::Span,
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

    /// Resolves one scale operator that does not need execution services.
    ///
    /// Returns `None` when the operator has a value-dependent implementation, so
    /// the caller keeps the step for the runtime instead of mis-folding it.
    fn context_free_operator(
        &self,
        operator: CoreBinaryOperator,
        left_operand: TypeId,
        right_operand: TypeId,
        span: syntax::Span,
    ) -> Result<Option<&language_core::BinaryOperatorDescriptor>, CompileError> {
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

    /// Compiles one conversion whose source subtype is only known at runtime.
    ///
    /// The array specification requires an extracted element to behave exactly
    /// like the stored value, so a conversion cannot resolve against the
    /// declared base alone. The compiler resolves the declared target for every
    /// candidate source subtype into a dispatch table, and the runtime selects
    /// the plan from the value's own subtype with one slot computation.
    pub(super) fn dynamic_convert(
        &self,
        expression: TypedExpression,
        source: ValueType,
        target_base: Option<TypeId>,
        target_subtype: Option<SubtypeId>,
        target_description: String,
        span: syntax::Span,
    ) -> Result<TypedExpression, CompileError> {
        let width = self.registry.subtype_dispatch_width();
        let slots = self.operand_dispatch_slots(width, source.subtype);
        let mut plans = Vec::with_capacity(width);
        let mut reference_output: Option<ValueType> = None;
        let mut dynamic_result = false;
        let mut plain_error: Option<CompileError> = None;
        for slot in &slots {
            let plan = match slot {
                Some(candidate) => {
                    let candidate_source = ValueType {
                        base: source.base,
                        subtype: *candidate,
                    };
                    match self.resolve_conversion_plan(
                        candidate_source,
                        target_base,
                        target_subtype,
                        span,
                    ) {
                        Ok(plan) => {
                            match reference_output {
                                None => reference_output = Some(plan.conversion.output),
                                Some(output) if output != plan.conversion.output => {
                                    dynamic_result = true;
                                }
                                Some(_) => {}
                            }
                            Some(plan)
                        }
                        Err(error) => {
                            if candidate.is_none() {
                                plain_error = Some(error);
                            }
                            None
                        }
                    }
                }
                None => None,
            };
            plans.push(plan);
        }
        let Some(output) = reference_output else {
            return Err(plain_error.unwrap_or_else(|| {
                CompileError::new(
                    span,
                    format!("conversion to {target_description} is not defined"),
                )
            }));
        };
        Ok(TypedExpression {
            output: Some(output),
            kind: TypedExpressionKind::DynamicConvert {
                dispatch: Box::new(ir::TypedConversionDispatch {
                    target_description,
                    plans,
                    dynamic_result,
                }),
                expression: Box::new(expression),
            },
            span,
        })
    }

    /// Resolves one candidate source type into a conversion plan.
    ///
    /// The cases mirror the static conversion path exactly: a unit-only target
    /// keeps the source subtype, a type-only target keeps the source subtype and
    /// only casts the base, and a full target resolves the registered subtype
    /// conversion and casts the magnitude back to the requested base.
    fn resolve_conversion_plan(
        &self,
        source: ValueType,
        target_base: Option<TypeId>,
        target_subtype: Option<SubtypeId>,
        span: syntax::Span,
    ) -> Result<ir::TypedConversionPlan, CompileError> {
        match (target_base, target_subtype) {
            (Some(base), Some(subtype)) => {
                let resolved = self
                    .registry
                    .resolve_subtype_conversion(source, subtype)
                    .map_err(|error| CompileError::core(span, error))?;
                let output = ValueType::qualified(base, subtype);
                Ok(ir::TypedConversionPlan {
                    conversion: ResolvedSubtypeConversion {
                        output,
                        scale: resolved.scale,
                    },
                    target_base: Some(base),
                })
            }
            (Some(base), None) => Ok(ir::TypedConversionPlan {
                conversion: ResolvedSubtypeConversion {
                    output: ValueType {
                        base,
                        subtype: source.subtype,
                    },
                    scale: Scale::IDENTITY,
                },
                target_base: Some(base),
            }),
            (None, Some(subtype)) => {
                let resolved = self
                    .registry
                    .resolve_subtype_conversion(source, subtype)
                    .map_err(|error| CompileError::core(span, error))?;
                Ok(ir::TypedConversionPlan {
                    conversion: resolved,
                    target_base: None,
                })
            }
            (None, None) => unreachable!("a conversion always names a type or a unit"),
        }
    }
}

/// Describes how the candidate complete types of one dynamic operation vary.
///
/// Which operands the compiler could not type is a compile-time fact. It
/// decides the dispatch table's shape and which candidate subtype each slot
/// addresses, keeping the runtime selection to one slot computation.
#[derive(Clone, Copy)]
pub(super) enum DynamicOperands {
    /// Only the left operand's subtype varies at runtime.
    Left,
    /// Only the right operand's subtype varies at runtime.
    Right,
    /// Both operands' subtypes vary independently at runtime.
    Both,
    /// The left operand is a generated literal that mirrors the right subtype.
    ///
    /// A negation compiles to `0 - operand`, and the zero literal must adopt the
    /// operand's subtype so the same-subtype rule applies. The literal stays a
    /// single runtime value, so only the right operand contributes slots.
    MirroredLeft,
}

impl DynamicOperands {
    /// Returns the dispatch table width of the left and right operand.
    fn dispatch_widths(self, width: usize) -> (usize, usize) {
        match self {
            Self::Left => (width, 1),
            Self::Right | Self::MirroredLeft => (1, width),
            Self::Both => (width, width),
        }
    }

    /// Returns the candidate complete types of one dispatch slot pair.
    fn candidate_types(
        self,
        left_type: ValueType,
        left_candidate: Option<SubtypeId>,
        right_type: ValueType,
        right_candidate: Option<SubtypeId>,
    ) -> (ValueType, ValueType) {
        let left = ValueType {
            base: left_type.base,
            subtype: left_candidate,
        };
        let right = ValueType {
            base: right_type.base,
            subtype: right_candidate,
        };
        match self {
            Self::MirroredLeft => (
                ValueType {
                    base: left_type.base,
                    subtype: right_candidate,
                },
                right,
            ),
            _ => (left, right),
        }
    }
}

#[cfg(test)]
#[path = "arrays.tests.rs"]
mod tests;
