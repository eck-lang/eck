//! Built-in array end operations: spellings, aliases, and compilation.

use crate::ir::{TypedExpression, TypedExpressionKind};
use crate::semantic::{ArrayElementMode, ArrayEndOperation, ArrayType, SemanticType};
use crate::syntax::Expression;

use crate::CompileError;

use crate::compiler::Compiler;

impl Compiler<'_> {
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
    pub(crate) fn compile_array_method(
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
            ArrayEndOperation::Push | ArrayEndOperation::Unshift => {
                let [value] = arguments else {
                    return Err(CompileError::new(
                        span,
                        format!("`{method_name}` expects exactly one value to add"),
                    ));
                };
                let element = self.compile_array_element(value, array_type)?;
                Some(self.prepare_element_for_storage(element, array_type)?)
            }
            ArrayEndOperation::Pop | ArrayEndOperation::Shift => {
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
        self.element_flow.remove(binding);
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
}

/// Every source spelling of a built-in array end operation.
///
/// This table binds source spellings to operations in one place, which makes
/// `append` exactly the operation of `push` and `prepend` exactly the
/// operation of `unshift`. The compiler emits the canonical operation for an
/// alias, so the runtime needs no second implementation, and diagnostics list
/// the supported spellings from the same table.
const ARRAY_METHODS: &[(&str, ArrayEndOperation)] = &[
    ("push", ArrayEndOperation::Push),
    ("append", ArrayEndOperation::Push),
    ("pop", ArrayEndOperation::Pop),
    ("unshift", ArrayEndOperation::Unshift),
    ("prepend", ArrayEndOperation::Unshift),
    ("shift", ArrayEndOperation::Shift),
];

/// Resolves one source method name to its built-in array end operation.
///
/// Returns `None` for a name no array operation claims, which lets the caller
/// report the supported spellings instead of a generic unknown-function error.
fn array_method(method_name: &str) -> Option<ArrayEndOperation> {
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
