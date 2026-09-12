//! Lexical variable scopes, local slot allocation, and nullability tracking.
//!
//! Every binding receives a statically allocated [`LocalVariableSlot`] and a
//! [`BindingId`] here, and this module owns the narrowing proofs that record
//! where a nullable binding has been proven non-null.

use ir::{BindingId, BindingMetadata, LocalVariableSlot, TypedExpression, TypedExpressionKind};
use language_core::ValueType;
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
    pub(super) fn expression_is_nullable(&self, expression: &TypedExpression) -> bool {
        matches!(
            expression.kind,
            TypedExpressionKind::Variable { nullable: true, .. }
        )
    }

    /// Rejects nullable operands before they reach concrete registry dispatch.
    pub(super) fn require_non_nullable_expression(
        &self,
        expression: &TypedExpression,
    ) -> Result<(), CompileError> {
        if self.expression_is_nullable(expression) {
            Err(CompileError::new(
                expression.span,
                "nullable value must be narrowed before this operation",
            ))
        } else {
            Ok(())
        }
    }
}
