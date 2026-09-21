//! The typed intermediate representation the compiler hands to the runtime.
//!
//! Every name, type, operator, comparison, and function is resolved before a
//! [`TypedProgram`] is produced, and the execution plans are precomputed so the
//! runtime never repeats resolution work on a hot path.

mod binding;
mod expression;
mod program;
mod statement;

pub use binding::{BindingContract, BindingId, BindingMetadata, LocalVariableSlot};
pub use expression::{
    CompleteTypeDomain, TypedBinaryDispatch, TypedBinaryExecutionPlan, TypedBinaryPlan,
    TypedComparisonDispatch, TypedComparisonPlan, TypedConversionDispatch, TypedConversionPlan,
    TypedExpression, TypedExpressionKind, TypedIndexDispatch, TypedOpenBinaryDispatch,
    TypedOpenComparisonDispatch, TypedScalePlan, TypedScaleStep, TypedUnaryNegationDispatch,
    TypedUnaryNegationPlan,
};
pub use program::TypedProgram;
pub use statement::{TypedBlock, TypedRangePlan, TypedStatement};
