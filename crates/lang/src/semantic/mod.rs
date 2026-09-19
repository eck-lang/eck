//! Semantic vocabulary and registry for an ECK language instance.

mod array;
mod configuration;
mod descriptor;
mod error;
mod extension;
mod ids;
mod operator;
pub mod registry;
mod subtype;
mod types;

pub use crate::values::Value;
pub use array::{ArrayElementMode, ArrayType};
pub use configuration::{
    ArrayValueFormatter, ConfigurationDescriptor, ConfigurationNormalizer, ConfigurationOverride,
    ConfigurationValue, ConfiguredValueFormatter, ConfiguredValueTransformer, ExecutionContext,
    OwnedConfiguredValueTransformer, RuntimeConfiguration, TypeConfigurationDescriptor,
};
pub use descriptor::{
    BinaryOperatorDescriptor, BinaryOperatorExecutor, BooleanEvaluator, ComparisonDescriptor,
    ComparisonExecutor, ContextBinaryOperatorExecutor, FunctionDescriptor, FunctionSignature,
    InPlaceBinaryOperatorExecutor, IndexExtractor, LiteralParser, NamespaceSymbol, NativeFunction,
    TypeDescriptor, ValueFormatter,
};
pub use error::CoreError;
pub use extension::Extension;
pub use ids::{ComparisonId, FunctionId, OperatorId, SubtypeId, TypeId};
pub use operator::{BinaryOperator, ComparisonOperator};
pub use registry::Registry;
pub use subtype::{
    ResolvedBinaryOperator, ResolvedComparison, ResolvedSubtypeConversion, Scale,
    SubtypeBinaryRule, SubtypeComparisonRule, SubtypeDescriptor, SubtypeRelativeRule, ValueType,
};
pub use types::SemanticType;

pub use registry::bootstrap::{default_registry, register_all};
