use crate::semantic::{
    BinaryOperator, ComparisonId, ComparisonOperator, ExecutionContext, FunctionId, OperatorId,
    SemanticType, TypeId, Value,
};

pub type LiteralParser = fn(&str, TypeId) -> Result<Value, crate::semantic::CoreError>;
pub type ValueFormatter = fn(&Value) -> Result<String, crate::semantic::CoreError>;
pub type BooleanEvaluator = fn(&Value) -> Result<bool, crate::semantic::CoreError>;
pub type BinaryOperatorExecutor = fn(&Value, &Value) -> Result<Value, crate::semantic::CoreError>;
/// Mutates an exclusively owned left operand with one same-type binary operation.
///
/// Implementations must leave the left operand as the operation result and may
/// only be registered when the result has the same base type as the left
/// operand. The runtime uses this optional contract only after proving that no
/// later expression can observe the original value.
pub type InPlaceBinaryOperatorExecutor =
    fn(&mut Value, &Value) -> Result<(), crate::semantic::CoreError>;
/// Registry-aware binary operator implementation with access to execution services.
///
/// The context exposes the owning registry so an executor can resolve related
/// types at execution time. Extensions use this contract when the result type
/// depends on runtime values, for example promoting an overflowed fixed-width
/// computation to a wider representation that is only known by name.
pub type ContextBinaryOperatorExecutor =
    for<'a> fn(&ExecutionContext<'a>, &Value, &Value) -> Result<Value, crate::semantic::CoreError>;
pub type ComparisonExecutor = fn(&Value, &Value) -> Result<bool, crate::semantic::CoreError>;
pub type NativeFunction = for<'a> fn(
    &ExecutionContext<'a>,
    &[Value],
) -> Result<Option<Value>, crate::semantic::CoreError>;

/// Describes whether a native function may produce externally visible effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionPurity {
    /// The function's purity has not been declared.
    Unknown,
    /// The function does not produce externally visible side effects.
    Pure,
    /// The function may produce externally visible side effects.
    Impure,
}

/// Describes whether equal inputs guarantee equal function results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionDeterminism {
    /// The function's determinism has not been declared.
    Unknown,
    /// The function returns the same result for equal inputs and state.
    Deterministic,
    /// The function's result may vary independently of its arguments.
    Nondeterministic,
}

/// Describes a category of external state a function may access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionExternalEffect {
    /// External effects have not been declared.
    Unknown,
    /// The function does not access external state.
    None,
    /// The function reads external state.
    ReadsExternalState,
    /// The function writes external state.
    WritesExternalState,
}

/// Summarizes the observable effects and failure behavior of a native function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionEffectSummary {
    /// Whether the function produces externally visible side effects.
    pub purity: FunctionPurity,
    /// Whether equal arguments guarantee an equal result.
    pub determinism: FunctionDeterminism,
    /// Whether the function can return an error for validly typed arguments.
    pub may_fail: bool,
    /// Whether the function accesses state outside the ECK value model.
    pub external_effect: FunctionExternalEffect,
}

impl FunctionEffectSummary {
    /// The conservative summary assigned to registrations without a declaration.
    pub const UNKNOWN: Self = Self {
        purity: FunctionPurity::Unknown,
        determinism: FunctionDeterminism::Unknown,
        may_fail: true,
        external_effect: FunctionExternalEffect::Unknown,
    };

    /// Describes a deterministic, pure function that cannot fail or access external state.
    pub const PURE: Self = Self {
        purity: FunctionPurity::Pure,
        determinism: FunctionDeterminism::Deterministic,
        may_fail: false,
        external_effect: FunctionExternalEffect::None,
    };
}
/// Converts one opaque integer value into a zero-based array index.
///
/// Integer type extensions register this contract so array indexing can read a
/// user-supplied index without knowing how the index's type stores its
/// magnitude. An implementation must reject a negative or out-of-range
/// magnitude with a runtime error instead of truncating it, because a
/// truncated index would silently address the wrong element.
pub type IndexExtractor = fn(&Value) -> Result<usize, crate::semantic::CoreError>;

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
    /// Parameter names in callback order; absent for legacy positional-only registrations.
    pub parameter_names: Option<Vec<&'static str>>,
    /// Compile-time literals substituted for omitted optional parameters.
    pub parameter_defaults: Vec<Option<Value>>,
    pub output: Option<SemanticType>,
    /// Explicit or conservative effect metadata for this overload.
    pub effect_summary: FunctionEffectSummary,
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
