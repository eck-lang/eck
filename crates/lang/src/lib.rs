//! The ECK language implementation.

pub mod analysis;
pub mod compiler;
pub mod connectors;
pub mod containers;
pub mod ir;
pub mod measures;
pub mod parser;
pub mod primitives;
pub mod runtime;
pub mod semantic;
pub mod std;
pub mod syntax;
pub mod values;

pub use crate::runtime::{ExecutionOptions, Executor, RuntimeError, execute};
pub use compiler::{CompileError, compile};
pub use containers::array::{ArrayEndOperation, ArrayType, ArrayValue};
pub use ir::{TypedExpression, TypedExpressionKind, TypedProgram, TypedStatement};
pub use measures::{data, frequency, linear, mass, percentage, time, volume};
pub use parser::{ECK_KEYWORDS, ParseError, parse};
pub use primitives::{
    DecimalExtension, DoubleExtension, FloatExtension, IntegerExtension, RegexExtension,
    StringExtension,
};
pub use semantic::{
    BinaryOperator, FunctionDeterminism, FunctionEffectSummary, FunctionExternalEffect,
    FunctionPurity, FunctionSignature, Registry, ScalarRepresentation, SubtypeDescriptor,
};
pub use semantic::{default_registry, register_all};
