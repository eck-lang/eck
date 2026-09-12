mod compiler;
mod error;

pub use compiler::compile;
pub use error::CompileError;
pub use ir::{
    LocalVariableSlot, TypedBinaryExecutionPlan, TypedBlock, TypedExpression, TypedExpressionKind,
    TypedProgram, TypedScalePlan, TypedScaleStep, TypedStatement,
};
