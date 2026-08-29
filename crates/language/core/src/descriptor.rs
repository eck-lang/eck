use crate::{BinaryOperator, FunctionId, OperatorId, Registry, TypeId, Value};

pub type LiteralParser = fn(&str, TypeId) -> Result<Value, crate::CoreError>;
pub type ValueFormatter = fn(&Value) -> Result<String, crate::CoreError>;
pub type BinaryOperatorExecutor = fn(&Value, &Value) -> Result<Value, crate::CoreError>;
pub type NativeFunction = fn(&Registry, &[Value]) -> Result<Option<Value>, crate::CoreError>;

#[derive(Clone)]
pub struct TypeDescriptor {
    pub id: TypeId,
    pub name: &'static str,
    pub parse_numeric_literal: Option<LiteralParser>,
    pub parse_string_literal: Option<LiteralParser>,
    pub parse_boolean_literal: Option<LiteralParser>,
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
