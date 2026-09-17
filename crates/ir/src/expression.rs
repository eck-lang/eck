use language_core::{
    BinaryOperator, ComparisonOperator, FunctionId, IndexExtractor, OperatorId,
    ResolvedBinaryOperator, ResolvedComparison, ResolvedSubtypeConversion, TypeId, Value,
    ValueType,
};
use syntax::{LogicalOperator, Span};

use crate::binding::{BindingId, LocalVariableSlot};

/// The compile-time element contract of one array value.
///
/// The contract is fixed when the array is declared or inferred and never
/// changes at runtime, so element access and element assignment can be typed
/// without inspecting the stored elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayType {
    /// The declared or inferred common element type.
    ///
    /// `subtype` is `Some` only when the array constrains the element subtype,
    /// such as `int<mm>[]`. For an unconstrained array the base type is the
    /// contract while each stored element keeps its own subtype.
    pub element: ValueType,
    /// Allows adaptive `int` elements to widen beyond their declared width.
    ///
    /// True only for an `int[]` element spelled with the adaptive `int` alias
    /// or inferred from integer literals. A fixed-width array such as `int64[]`
    /// keeps `false`, so storing a value that exceeds the declared width is
    /// rejected instead of widening the element.
    pub adaptive_integer: bool,
}

/// One built-in operation on the end of an array value.
///
/// The compiler resolves a source method name to one of these variants, so a
/// spelling such as `append` or `prepend` is an alias of the canonical
/// operation and never gains a separate implementation, semantic rule, or
/// runtime step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayMethod {
    /// Adds one value after the last element (`push`, `append`).
    Push,
    /// Removes and returns the last element, or null when the array is empty.
    Pop,
    /// Adds one value before the first element (`unshift`, `prepend`).
    Unshift,
    /// Removes and returns the first element, or null when the array is empty.
    Shift,
}

impl ArrayMethod {
    /// Returns the canonical source name of this operation.
    ///
    /// Diagnostics use the canonical name so an alias reports the operation the
    /// runtime actually performs.
    pub fn name(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Pop => "pop",
            Self::Unshift => "unshift",
            Self::Shift => "shift",
        }
    }

    /// Reports whether this operation removes one element instead of adding one.
    ///
    /// A removal produces a nullable element, because an empty array has none
    /// to remove, while an addition produces no value at all.
    pub fn removes_element(self) -> bool {
        matches!(self, Self::Pop | Self::Shift)
    }
}

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

impl TypedExpression {
    /// Returns this expression's array contract when it produces an array.
    pub fn array_type(&self) -> Option<ArrayType> {
        match &self.kind {
            TypedExpressionKind::ArrayLiteral { array_type, .. }
            | TypedExpressionKind::Variable {
                array_type: Some(array_type),
                ..
            } => Some(*array_type),
            _ => None,
        }
    }

    /// Reports whether this expression may produce the null value.
    ///
    /// A nullable binding yields null until a lexical proof narrows it, and a
    /// removal from an array yields null when there is no element to remove.
    pub fn is_nullable(&self) -> bool {
        match &self.kind {
            TypedExpressionKind::Variable { nullable, .. } => *nullable,
            TypedExpressionKind::ArrayMethod { method, .. } => method.removes_element(),
            _ => false,
        }
    }

    /// Returns whether this expression's complete value type is only known at runtime.
    ///
    /// An unconstrained array element keeps the subtype it was stored with, so a
    /// read of it cannot be typed statically. The compiler records that here so
    /// an enclosing operation can pre-resolve one plan per candidate subtype and
    /// let the runtime select the plan from the value it actually holds.
    pub fn dynamic_complete_type(&self) -> bool {
        match &self.kind {
            TypedExpressionKind::Variable {
                dynamic_complete_type,
                ..
            } => *dynamic_complete_type,
            TypedExpressionKind::ArrayMethod { dynamic_result, .. } => *dynamic_result,
            TypedExpressionKind::ElementAccess {
                dynamic_subtype, ..
            } => *dynamic_subtype,
            TypedExpressionKind::DynamicBinary { dynamic_result, .. } => *dynamic_result,
            TypedExpressionKind::DynamicConvert { dispatch, .. } => dispatch.dynamic_result,
            TypedExpressionKind::ElementStore { expression, .. } => {
                expression.dynamic_complete_type()
            }
            _ => false,
        }
    }
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

/// One pre-resolved binary plan for a candidate pair of complete operand types.
#[derive(Clone)]
pub struct TypedBinaryPlan {
    pub resolution: ResolvedBinaryOperator,
    pub execution_plan: TypedBinaryExecutionPlan,
}

/// Pre-resolved binary plans for an operation whose operand complete types are
/// not fully known at compile time.
///
/// An unconstrained array element carries whatever subtype it was stored with,
/// so the compiler resolves the operation once per candidate subtype and stores
/// the plans here. Each operand contributes either one dispatch slot, when its
/// complete type is known, or [`Registry::subtype_dispatch_width`] slots, when
/// its subtype is selected by its runtime value. Plans are stored row-major and
/// a `None` entry marks a candidate pair the registry does not define, which the
/// runtime reports only if that pair is actually reached.
#[derive(Clone)]
pub struct TypedBinaryDispatch {
    pub left_width: usize,
    pub right_width: usize,
    pub plans: Vec<Option<TypedBinaryPlan>>,
}

/// Pre-resolved comparison relations for an operation whose operand complete
/// types are not fully known at compile time.
///
/// The layout mirrors [`TypedBinaryDispatch`]: each operand contributes one slot
/// when its complete type is known and the registry's subtype dispatch width
/// otherwise, and a `None` entry marks a candidate pair with no registered
/// relation.
#[derive(Clone)]
pub struct TypedComparisonDispatch {
    pub left_width: usize,
    pub right_width: usize,
    pub resolutions: Vec<Option<ResolvedComparison>>,
}

/// One pre-resolved subtype conversion for a candidate source subtype.
#[derive(Clone, Copy)]
pub struct TypedConversionPlan {
    /// The conversion scale and the complete type it produces.
    pub conversion: ResolvedSubtypeConversion,
    /// An optional base type cast applied after scaling.
    pub target_base: Option<TypeId>,
}

/// Pre-resolved conversions for a value whose source subtype varies at runtime.
///
/// The table is addressed by the source's subtype dispatch slot, so a dynamic
/// conversion selects its plan arithmetically instead of resolving a subtype
/// conversion during execution. A `None` entry marks a source subtype with no
/// registered conversion to the declared target.
#[derive(Clone)]
pub struct TypedConversionDispatch {
    /// Describes the declared target for diagnostics, such as `` `->to(mm)` ``.
    pub target_description: String,
    pub plans: Vec<Option<TypedConversionPlan>>,
    /// Reports whether the result's complete type varies with the source subtype.
    pub dynamic_result: bool,
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
        array_type: Option<ArrayType>,
        /// Reports whether the binding may hold a subtype only known at runtime.
        dynamic_complete_type: bool,
    },
    /// Builds one contiguous array payload from evaluated element expressions.
    ArrayLiteral {
        array_type: ArrayType,
        elements: Vec<TypedExpression>,
    },
    /// Applies one built-in end operation to the array stored in a local slot.
    ///
    /// The receiver is the array binding itself rather than a value, because the
    /// operation mutates that binding. An adding operation carries its single
    /// stored value in `arguments`; a removing operation has no argument. The
    /// stored value is prepared through the same element contract as any other
    /// value that enters the array, so insertion never has a coercion path of
    /// its own, and a removal produces the element together with whatever
    /// subtype it was stored with.
    ArrayMethod {
        method: ArrayMethod,
        binding: BindingId,
        slot: LocalVariableSlot,
        arguments: Vec<TypedExpression>,
        /// Reports whether the produced element's subtype is only known at runtime.
        dynamic_result: bool,
        /// The value a removal produces when the array holds no element.
        ///
        /// The compiler resolves the language's null value once, so an empty
        /// removal never repeats a null-literal lookup or parse during
        /// execution. An adding operation produces no value at all and
        /// therefore carries `None`.
        empty_result: Option<Value>,
    },
    /// Applies an array element representation contract to one evaluated value.
    ///
    /// This node marks the exact point where a value crosses from the
    /// representation its expression produced into the representation the
    /// destination array declared. `element` is that destination contract: the
    /// complete element type every stored value must carry, whose base
    /// representation the runtime enforces and whose optional subtype the stored
    /// value keeps.
    ///
    /// The compiler inserts the node only when the crossing needs runtime work.
    /// An element it proved already carries the declared representation, and
    /// every adaptive `int` element, cross into storage without it. The node
    /// belongs to the stored expression, so a literal initializer and an indexed
    /// assignment reach their destination through the same prepared contract.
    ElementStore {
        element: ValueType,
        expression: Box<TypedExpression>,
    },
    /// Reads one zero-based element, using an immediate index when available.
    ElementAccess {
        array: Box<TypedExpression>,
        index: Box<TypedExpression>,
        constant_index: Option<usize>,
        index_extractor: IndexExtractor,
        /// Reports whether the stored element's subtype is only known at runtime.
        dynamic_subtype: bool,
    },
    /// Applies the plan selected by the operands' runtime complete types.
    ///
    /// The compiler pre-resolved every candidate subtype into `dispatch`, so the
    /// runtime selects a plan with one slot computation instead of resolving the
    /// subtype rules again.
    DynamicBinary {
        operator: BinaryOperator,
        dispatch: Box<TypedBinaryDispatch>,
        /// Reports whether the result's complete type also varies at runtime.
        dynamic_result: bool,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Applies the relation selected by the operands' runtime complete types.
    DynamicComparison {
        operator: ComparisonOperator,
        dispatch: Box<TypedComparisonDispatch>,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
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
    /// Applies the conversion the source's runtime subtype selects.
    DynamicConvert {
        dispatch: Box<TypedConversionDispatch>,
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
