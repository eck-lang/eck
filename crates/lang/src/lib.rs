//! The ECK language implementation.

pub mod compiler;
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

pub use crate::runtime::{RuntimeError, execute};
pub use compiler::{CompileError, compile};
pub use containers::array::{ArrayElementMode, ArrayEndOperation, ArrayType, ArrayValue};
pub use ir::{TypedExpression, TypedExpressionKind, TypedProgram, TypedStatement};
pub use measures::{data, frequency, linear, mass, percentage, time, volume};
pub use parser::{ECK_KEYWORDS, ParseError, parse};
pub use primitives::{
    DecimalExtension, DoubleExtension, FloatExtension, IntegerExtension, RegexExtension,
    StringExtension,
};
pub use semantic::{
    ArrayElementMode as SemanticArrayElementMode, BinaryOperator, FunctionSignature, Registry,
    SubtypeDescriptor,
};
pub use semantic::{default_registry, register_all};
