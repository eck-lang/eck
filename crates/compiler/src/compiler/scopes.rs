//! Lexical variable scopes, local slot allocation, and nullability tracking.
//!
//! Every binding receives a statically allocated [`LocalVariableSlot`] and a
//! [`BindingId`] here, and this module owns the narrowing proofs that record
//! where a nullable binding has been proven non-null.

use ir::{ArrayType, BindingId, BindingMetadata, LocalVariableSlot, TypedExpression};
use language_core::{FunctionId, FunctionSignature, ValueType};
use syntax::{ComparisonOperator, Expression};

use crate::CompileError;

use super::{Compiler, LocalVariable};

impl Compiler<'_> {
    /// Allocates a binding identity and runtime slot in the active lexical scope.
    pub(super) fn bind_local_variable(
        &mut self,
        name: String,
        value_type: ValueType,
        mutable: bool,
        nullable: bool,
        array_type: Option<ArrayType>,
        dynamic_complete_type: bool,
        declaration_span: syntax::Span,
    ) -> LocalVariable {
        let slot = LocalVariableSlot(self.next_local_slot);
        self.next_local_slot += 1;
        let binding = BindingId(self.bindings.len());
        let variable = LocalVariable {
            binding,
            value_type,
            slot,
            mutable,
            nullable,
            array_type,
            dynamic_complete_type,
        };
        self.bindings.push(BindingMetadata {
            id: binding,
            slot,
            name: name.clone(),
            mutable,
            value_type,
            nullable,
            declaration_span,
            scope_depth: self.variable_scopes.len() - 1,
        });
        self.variable_scopes
            .last_mut()
            .expect("compiler always has a variable scope")
            .insert(name, variable);
        variable
    }

    /// Reports whether the active lexical scope already owns `name`.
    pub(super) fn variable_declared_in_current_scope(&self, name: &str) -> bool {
        self.variable_scopes
            .last()
            .expect("compiler always has a variable scope")
            .contains_key(name)
    }

    /// Finds the nearest active declaration for a source variable name.
    pub(super) fn resolve_variable(&self, name: &str) -> Option<LocalVariable> {
        self.variable_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    /// Marks the nearest binding for `name` as carrying a runtime complete type.
    ///
    /// A statement that assigns an extracted element to an existing binding must
    /// keep dispatching on the stored subtype, because the binding now holds the
    /// element's own complete type rather than the declared base alone.
    pub(super) fn mark_variable_dynamic(&mut self, name: &str) {
        for scope in self.variable_scopes.iter_mut().rev() {
            if let Some(variable) = scope.get_mut(name) {
                variable.dynamic_complete_type = true;
                return;
            }
        }
    }

    /// Finds the binding narrowed by a direct `binding != null` condition.
    pub(super) fn non_null_narrowing_binding(&self, condition: &Expression) -> Option<BindingId> {
        let Expression::Comparison {
            operator: ComparisonOperator::NotEqual,
            left_operand,
            right_operand,
            ..
        } = condition
        else {
            return None;
        };
        let name = match (left_operand.as_ref(), right_operand.as_ref()) {
            (Expression::Variable { name, .. }, Expression::Null { .. })
            | (Expression::Null { .. }, Expression::Variable { name, .. }) => name,
            _ => return None,
        };
        self.resolve_variable(name)
            .filter(|variable| variable.nullable)
            .map(|variable| variable.binding)
    }

    /// Adds one lexical proof that a nullable binding is currently non-null.
    pub(super) fn push_narrowing(&mut self, binding: BindingId) {
        *self.narrowed_bindings.entry(binding).or_default() += 1;
    }

    /// Removes one lexical non-null proof while preserving any enclosing proof.
    pub(super) fn pop_narrowing(&mut self, binding: BindingId) {
        let Some(proof_count) = self.narrowed_bindings.get_mut(&binding) else {
            // An assignment inside the narrowed region invalidates every active
            // proof for this binding. Leaving that region must then be a no-op.
            return;
        };
        *proof_count -= 1;
        if *proof_count == 0 {
            self.narrowed_bindings.remove(&binding);
        }
    }

    /// Requires the configured plain boolean type for ordinary logical operations.
    pub(super) fn require_plain_boolean(
        &self,
        value_type: ValueType,
        span: syntax::Span,
    ) -> Result<(), CompileError> {
        let expected = ValueType::plain(
            self.registry
                .default_boolean()
                .map_err(|error| CompileError::core(span, error))?,
        );
        if value_type == expected {
            Ok(())
        } else {
            Err(CompileError::new(
                span,
                format!(
                    "expected `{}`, found `{}`",
                    self.registry.value_type_name(expected),
                    self.registry.value_type_name(value_type)
                ),
            ))
        }
    }

    /// Reports whether an expression may evaluate to the null value.
    ///
    /// A nullable binding yields null until a lexical proof narrows it, and a
    /// removal from an array yields null when there is no element to remove.
    /// The typed expression owns this decision so every consumer, including
    /// binding, assignment, and null-check compilation, asks one implementation.
    pub(super) fn expression_is_nullable(&self, expression: &TypedExpression) -> bool {
        expression.is_nullable()
    }

    /// Rejects nullable and array operands before concrete registry dispatch.
    ///
    /// Scalar operations, conditions, calls, and conversions all require one
    /// complete scalar value. An array is a container whose compile-time
    /// contract lives in its element type, so passing it where a scalar is
    /// expected is always a type error rather than an operator lookup failure.
    pub(super) fn require_scalar_expression(
        &self,
        expression: &TypedExpression,
    ) -> Result<(), CompileError> {
        self.require_non_nullable_expression(expression)?;
        if expression.array_type().is_some() {
            return Err(CompileError::new(
                expression.span,
                "an array cannot be used as a scalar operand",
            ));
        }
        Ok(())
    }

    /// Rejects a nullable value where one complete value is required.
    ///
    /// Nullability is a property of the value, not of the operation, so this
    /// check stays in place for every consumer even when the consumer can accept
    /// a container. A value that may be null must be narrowed first.
    pub(super) fn require_non_nullable_expression(
        &self,
        expression: &TypedExpression,
    ) -> Result<(), CompileError> {
        if self.expression_is_nullable(expression) {
            return Err(CompileError::new(
                expression.span,
                "nullable value must be narrowed before this operation",
            ));
        }
        Ok(())
    }

    /// Rejects a container argument the called native function cannot receive.
    ///
    /// An array reaches a native function as the container itself, so the bare
    /// element base type must not silently select an overload that expects one
    /// scalar value: the function would read a payload that the array does not
    /// carry. Only a signature that accepts any single value may receive a
    /// container, which is how a generic operation such as `print` renders one.
    pub(super) fn require_scalar_arguments(
        &self,
        function: FunctionId,
        arguments: &[TypedExpression],
        span: syntax::Span,
    ) -> Result<(), CompileError> {
        if arguments
            .iter()
            .all(|argument| argument.array_type().is_none())
        {
            return Ok(());
        }
        let descriptor = self
            .registry
            .function(function)
            .map_err(|error| CompileError::core(span, error))?;
        if matches!(descriptor.signature, FunctionSignature::AnySingle) {
            return Ok(());
        }
        let container = arguments
            .iter()
            .find(|argument| argument.array_type().is_some())
            .expect("an array argument was found above");
        Err(CompileError::new(
            container.span,
            "an array cannot be used as a scalar operand",
        ))
    }
}
