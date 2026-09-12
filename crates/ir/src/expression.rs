use language_core::{
    FunctionId, OperatorId, ResolvedBinaryOperator, ResolvedComparison, ResolvedSubtypeConversion,
    TypeId, Value, ValueType,
};
use syntax::{LogicalOperator, Span};

use crate::binding::{BindingId, LocalVariableSlot};

/// One typed expression node with its inferred output type and source span.
///
/// `output` is `None` when the expression produces no value, such as a call to
/// a function that returns `void`.
#[derive(Clone)]
pub struct TypedExpression {
    pub kind: TypedExpressionKind,
    pub output: Option<ValueType>,
    pub span: Span,
}

/// One compile-time-resolved arithmetic step used to scale a magnitude.
///
/// `factor` comes from a resolved literal and `operator` selects the
/// multiplication or division that applies it.
#[derive(Clone)]
pub struct TypedScaleStep {
    pub operator: OperatorId,
    pub factor: Value,
}

/// Stores the multiplication and division needed for one subtype magnitude scale.
#[derive(Clone, Default)]
pub struct TypedScalePlan {
    pub numerator: Option<TypedScaleStep>,
    pub denominator: Option<TypedScaleStep>,
}

impl TypedScalePlan {
    /// Returns whether this plan leaves the source magnitude unchanged.
    pub fn is_identity(&self) -> bool {
        self.numerator.is_none() && self.denominator.is_none()
    }
}

/// All pre-resolved dispatch needed to execute one typed binary expression.
///
/// It carries the operand scaling and optional relative adjustment so the
/// runtime never resolves a subtype conversion or operator during evaluation.
#[derive(Clone)]
pub struct TypedBinaryExecutionPlan {
    pub left_operand_scale: TypedScalePlan,
    pub right_operand_scale: TypedScalePlan,
    pub relative_adjustment_operator: Option<OperatorId>,
}

/// The resolved operation or value behind one [`TypedExpression`].
#[derive(Clone)]
pub enum TypedExpressionKind {
    /// A constant runtime value.
    Literal(Value),
    /// Reads a local binding from its statically allocated slot.
    Variable {
        name: String,
        binding: BindingId,
        slot: LocalVariableSlot,
        nullable: bool,
    },
    /// Applies a resolved binary operator, including operand scaling.
    Binary {
        resolution: ResolvedBinaryOperator,
        execution_plan: Box<TypedBinaryExecutionPlan>,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Applies a resolved comparison relation and yields a boolean.
    Comparison {
        resolution: ResolvedComparison,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Tests an operand against null, either `==` or `!=`.
    NullCheck {
        operand: Box<TypedExpression>,
        equal: bool,
        boolean_type: TypeId,
    },
    /// Short-circuit `&&` or `||`.
    Logical {
        operator: LogicalOperator,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Boolean negation.
    LogicalNot { operand: Box<TypedExpression> },
    /// Applies a resolved subtype conversion and optional base-type change.
    Convert {
        conversion: ResolvedSubtypeConversion,
        target_base: Option<TypeId>,
        expression: Box<TypedExpression>,
    },
    /// Calls a resolved function with positional arguments.
    Call {
        function: FunctionId,
        arguments: Vec<TypedExpression>,
    },
    /// Pipes a base value into a resolved function as its first argument.
    Pipe {
        function: FunctionId,
        base: Box<TypedExpression>,
        arguments: Vec<TypedExpression>,
    },
}
