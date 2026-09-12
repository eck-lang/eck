use crate::{
    BinaryOperator, ComparisonId, ComparisonOperator, ExecutionContext, FunctionId, OperatorId,
    TypeId, Value,
};

pub type LiteralParser = fn(&str, TypeId) -> Result<Value, crate::CoreError>;
pub type ValueFormatter = fn(&Value) -> Result<String, crate::CoreError>;
pub type BooleanEvaluator = fn(&Value) -> Result<bool, crate::CoreError>;
pub type BinaryOperatorExecutor = fn(&Value, &Value) -> Result<Value, crate::CoreError>;
/// Mutates an exclusively owned left operand with one same-type binary operation.
///
/// Implementations must leave the left operand as the operation result and may
/// only be registered when the result has the same base type as the left
/// operand. The runtime uses this optional contract only after proving that no
/// later expression can observe the original value.
pub type InPlaceBinaryOperatorExecutor = fn(&mut Value, &Value) -> Result<(), crate::CoreError>;
/// Registry-aware binary operator implementation with access to execution services.
///
/// The context exposes the owning registry so an executor can resolve related
/// types at execution time. Extensions use this contract when the result type
/// depends on runtime values, for example promoting an overflowed fixed-width
/// computation to a wider representation that is only known by name.
pub type ContextBinaryOperatorExecutor =
    for<'a> fn(&ExecutionContext<'a>, &Value, &Value) -> Result<Value, crate::CoreError>;
pub type ComparisonExecutor = fn(&Value, &Value) -> Result<bool, crate::CoreError>;
pub type NativeFunction =
    for<'a> fn(&ExecutionContext<'a>, &[Value]) -> Result<Option<Value>, crate::CoreError>;

#[derive(Clone)]
pub struct TypeDescriptor {
    pub id: TypeId,
    pub name: &'static str,
    /// Declares whether values of this type represent integral magnitudes.
    ///
    /// Integer-only language constructs, such as range iteration, use this
    /// semantic capability instead of inferring it from a particular literal
    /// spelling. Extension authors must set it only when every valid value of
    /// the type has no fractional component.
    pub is_integer: bool,
    pub parse_numeric_literal: Option<LiteralParser>,
    pub parse_string_literal: Option<LiteralParser>,
    pub parse_regex_literal: Option<LiteralParser>,
    pub parse_boolean_literal: Option<LiteralParser>,
    pub parse_null_literal: Option<LiteralParser>,
    pub format: ValueFormatter,
}

#[derive(Clone)]
pub struct BinaryOperatorDescriptor {
    pub id: OperatorId,
    pub operator: BinaryOperator,
    pub left_operand_type: TypeId,
    pub right_operand_type: TypeId,
    pub result_type: TypeId,
    pub execute: BinaryOperatorExecutor,
    /// Optional allocation-avoiding executor for an exclusively owned left operand.
    pub in_place_execute: Option<InPlaceBinaryOperatorExecutor>,
    /// Registry-aware override preferred by the runtime when present.
    ///
    /// The plain `execute` callback remains the context-free implementation so
    /// isolated unit tests and context-free dispatch keep working. The runtime
    /// calls `context_execute` instead whenever an extension registered one,
    /// which allows value-dependent behavior such as overflow promotion while
    /// the statically declared `result_type` still describes the common case.
    pub context_execute: Option<ContextBinaryOperatorExecutor>,
}

#[derive(Clone)]
pub struct ComparisonDescriptor {
    pub id: ComparisonId,
    pub operator: ComparisonOperator,
    pub left_operand_type: TypeId,
    pub right_operand_type: TypeId,
    pub execute: ComparisonExecutor,
}

/// Describes which argument base types a native function accepts.
#[derive(Clone, PartialEq, Eq)]
pub enum FunctionSignature {
    /// Matches one exact, ordered list of argument base types.
    Exact(Vec<TypeId>),
    /// Matches any call with exactly one argument when no exact overload exists.
    AnySingle,
}

#[derive(Clone)]
pub struct FunctionDescriptor {
    pub id: FunctionId,
    pub name: &'static str,
    pub signature: FunctionSignature,
    pub output: Option<TypeId>,
    pub execute: NativeFunction,
}

/// Describes one kind of symbol exported by a registered namespace.
///
/// Function exports refer to a registered overload family by its canonical
/// registry name. Additional variants can be added as constants, types, and
/// nested namespaces become compiler-visible symbols.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamespaceSymbol {
    /// Exposes a native function overload family under a namespace member name.
    Function { function_name: &'static str },
}
