use frame_model::{Frame, FrameType, RecordType, RelationBinding, RelationDefinition};
use language_core::{
    ConfigurationOverride, ResolvedBinaryOperator, ResolvedComparison, Value, ValueType,
};
use syntax::Span;

use crate::binding::{BindingId, LocalVariableSlot};
use crate::expression::TypedExpression;

/// A statement sequence that shares one lexical variable scope.
#[derive(Clone)]
pub struct TypedBlock {
    pub statements: Vec<TypedStatement>,
    pub span: Span,
}

/// Stores the compiled execution plan for one integer range loop.
///
/// The initial plan avoids registry resolution during ordinary iterations. The
/// runtime rebuilds it only if an overflow promotion changes the iteration
/// value's concrete type.
#[derive(Clone)]
pub struct TypedRangePlan {
    pub current_type: ValueType,
    pub increment_unit: Value,
    pub comparison: ResolvedComparison,
    pub increment: ResolvedBinaryOperator,
}

/// One executable statement whose bindings, operators, and functions the
/// compiler has already resolved.
#[derive(Clone)]
pub enum TypedStatement {
    /// Declares a structural row type.
    TypeDeclaration { definition: RecordType, span: Span },
    /// Declares a native frame, optionally initialized from a frame literal.
    FrameDeclaration {
        name: String,
        frame_type: FrameType,
        frame: Option<Frame>,
        span: Span,
    },
    /// Declares a reusable relation role and predicate definition.
    RelationDefinition {
        definition: RelationDefinition,
        span: Span,
    },
    /// Binds a frame variable to a relation definition.
    RelationBinding {
        binding: RelationBinding,
        span: Span,
    },
    /// Applies a validated `@config` override to subsequent statements.
    Configuration {
        configuration_override: ConfigurationOverride,
        span: Span,
    },
    /// Declares a local binding and stores its initializer in a slot.
    VariableDeclaration {
        name: String,
        binding: BindingId,
        slot: LocalVariableSlot,
        mutable: bool,
        value_type: ValueType,
        expression: TypedExpression,
        span: Span,
    },
    /// Stores a new value in an existing mutable binding slot.
    Assignment {
        name: String,
        binding: BindingId,
        slot: LocalVariableSlot,
        expression: TypedExpression,
        span: Span,
    },
    /// Executes a nested lexically scoped block.
    Block(TypedBlock),
    /// Executes one of two blocks based on a boolean condition.
    If {
        condition: TypedExpression,
        body: TypedBlock,
        else_body: Option<TypedBlock>,
        span: Span,
    },
    /// Repeats a block while a boolean condition stays true.
    While {
        condition: TypedExpression,
        body: TypedBlock,
        span: Span,
    },
    /// Iterates a block over an integer range with a precompiled step plan.
    For {
        variable: String,
        binding: BindingId,
        slot: LocalVariableSlot,
        variable_type: ValueType,
        start: TypedExpression,
        end: TypedExpression,
        range_plan: TypedRangePlan,
        body: TypedBlock,
        span: Span,
    },
    /// Exits the innermost enclosing loop.
    Break { span: Span },
    /// Skips to the next iteration of the innermost enclosing loop.
    Continue { span: Span },
    /// Evaluates an expression for its side effects and discards the result.
    Expression(TypedExpression),
}
