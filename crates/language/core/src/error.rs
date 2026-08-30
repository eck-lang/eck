use thiserror::Error;

use crate::{
    BinaryOperator, ComparisonId, ComparisonOperator, FunctionId, OperatorId, SubtypeId, TypeId,
};

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("type id {0:?} was not allocated by this registry")]
    UnallocatedTypeId(TypeId),
    #[error("type id {0:?} is already registered")]
    DuplicateTypeId(TypeId),
    #[error("type `{0}` is already registered")]
    DuplicateType(String),
    #[error("subtype id {0:?} was not allocated by this registry")]
    UnallocatedSubtypeId(SubtypeId),
    #[error("subtype id {0:?} is already registered")]
    DuplicateSubtypeId(SubtypeId),
    #[error("subtype `{0}` is already registered")]
    DuplicateSubtype(String),
    #[error("subtype `{0}` must register at least one literal suffix")]
    MissingLiteralSuffix(String),
    #[error("literal suffix `{0}` is already registered")]
    DuplicateLiteralSuffix(String),
    #[error("unknown numeric literal suffix `{0}`")]
    UnknownLiteralSuffix(String),
    #[error("unknown subtype id {0:?}")]
    UnknownSubtypeId(SubtypeId),
    #[error("scale denominator cannot be zero")]
    InvalidScale,
    #[error("unknown type id {0:?}")]
    UnknownTypeId(TypeId),
    #[error("unknown operator id {0:?}")]
    UnknownOperatorId(OperatorId),
    #[error("unknown comparison id {0:?}")]
    UnknownComparisonId(ComparisonId),
    #[error("unknown function id {0:?}")]
    UnknownFunctionId(FunctionId),
    #[error("default {0} type is not configured")]
    MissingDefault(&'static str),
    #[error("{literal_kind} literals are not supported by type `{type_name}`")]
    UnsupportedLiteral {
        type_name: String,
        literal_kind: &'static str,
    },
    #[error(
        "operator `{operator}` is already registered for `{left_operand_type}` and `{right_operand_type}`"
    )]
    DuplicateOperator {
        operator: BinaryOperator,
        left_operand_type: String,
        right_operand_type: String,
    },
    #[error(
        "comparison `{operator}` is already registered for `{left_operand_type}` and `{right_operand_type}`"
    )]
    DuplicateComparison {
        operator: ComparisonOperator,
        left_operand_type: String,
        right_operand_type: String,
    },
    #[error(
        "subtype operator `{operator}` is already registered for `{left_operand_subtype}` and `{right_operand_subtype}`"
    )]
    DuplicateSubtypeOperator {
        operator: BinaryOperator,
        left_operand_subtype: String,
        right_operand_subtype: String,
    },
    #[error(
        "subtype comparison `{operator}` is already registered for `{left_operand_subtype}` and `{right_operand_subtype}`"
    )]
    DuplicateSubtypeComparison {
        operator: ComparisonOperator,
        left_operand_subtype: String,
        right_operand_subtype: String,
    },
    #[error("subtype operator `{0}` cannot register a rule for two plain operands")]
    UnreachableSubtypeOperatorRule(BinaryOperator),
    #[error("subtype comparison `{0}` cannot register a rule for two plain operands")]
    UnreachableSubtypeComparisonRule(ComparisonOperator),
    #[error("conversion from subtype `{from}` to `{to}` is already registered")]
    DuplicateSubtypeConversion { from: String, to: String },
    #[error("conversion from subtype `{0}` to itself must use the identity scale")]
    InvalidIdentitySubtypeConversion(String),
    #[error("conversion from `{from}` to subtype `{to}` is not defined")]
    SubtypeConversionNotDefined { from: String, to: String },
    #[error(
        "operator `{operator}` is not defined for `{left_operand_type}` and `{right_operand_type}`"
    )]
    SubtypeOperatorNotDefined {
        operator: BinaryOperator,
        left_operand_type: String,
        right_operand_type: String,
    },
    #[error(
        "comparison `{operator}` is not defined for `{left_operand_type}` and `{right_operand_type}`"
    )]
    SubtypeComparisonNotDefined {
        operator: ComparisonOperator,
        left_operand_type: String,
        right_operand_type: String,
    },
    #[error(
        "operator `{operator}` is not defined for `{left_operand_type}` and `{right_operand_type}`"
    )]
    OperatorNotDefined {
        operator: BinaryOperator,
        left_operand_type: String,
        right_operand_type: String,
    },
    #[error(
        "comparison `{operator}` is not defined for `{left_operand_type}` and `{right_operand_type}`"
    )]
    ComparisonNotDefined {
        operator: ComparisonOperator,
        left_operand_type: String,
        right_operand_type: String,
    },
    #[error("unknown function `{0}`")]
    UnknownFunction(String),
    #[error("function `{name}` already has overload `{signature}`")]
    DuplicateFunctionSignature { name: String, signature: String },
    #[error("no overload of `{name}` accepts ({})", .arguments.join(", "))]
    NoMatchingFunction {
        name: String,
        arguments: Vec<String>,
    },
    #[error("invalid value representation for type `{0}`")]
    InvalidValueRepresentation(String),
    #[error("invalid literal `{raw_text}` for type `{type_name}`: {message}")]
    InvalidLiteral {
        raw_text: String,
        type_name: String,
        message: String,
    },
    #[error("division by zero")]
    DivisionByZero,
    #[error("runtime error: {0}")]
    Runtime(String),
}
