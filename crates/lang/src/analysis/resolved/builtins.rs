//! Verifies registered callback identities against a cached builtin inventory.

use std::sync::OnceLock;

use crate::semantic::{
    ComparisonId, IndexExtractor, OperatorId, Registry, TypeId, Value, ValueType,
};

/// Canonical callbacks are registered once; user registries have independent IDs.
static CANONICAL_REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Maps resolved registry identities onto verified builtin implementations.
pub(super) struct Builtins<'registry> {
    registry: &'registry Registry,
    canonical: &'registry Registry,
    dispatch_inventory_trusted: bool,
}

impl<'registry> Builtins<'registry> {
    /// Acquires the trusted builtin callback inventory without executing user code.
    pub(super) fn new(registry: &'registry Registry) -> Self {
        if registry.builtin_inventory_is_certified() {
            return Self {
                registry,
                canonical: registry,
                dispatch_inventory_trusted: true,
            };
        }
        let mut builtins = Self {
            registry,
            canonical: CANONICAL_REGISTRY.get_or_init(|| {
                crate::semantic::default_registry().expect("builtin registration is valid")
            }),
            dispatch_inventory_trusted: false,
        };
        builtins.dispatch_inventory_trusted = builtins.dispatch_inventory();
        builtins
    }

    /// Matches parser and formatter callback identities before using a builtin name.
    fn canonical_type(&self, type_id: TypeId) -> Option<TypeId> {
        let actual = self.registry.type_descriptor(type_id).ok()?;
        let canonical_id = self.canonical.type_by_name(actual.name)?;
        let expected = self.canonical.type_descriptor(canonical_id).ok()?;
        let parser_matches = [
            (actual.parse_numeric_literal, expected.parse_numeric_literal),
            (actual.parse_string_literal, expected.parse_string_literal),
            (actual.parse_regex_literal, expected.parse_regex_literal),
            (actual.parse_boolean_literal, expected.parse_boolean_literal),
            (actual.parse_null_literal, expected.parse_null_literal),
        ]
        .into_iter()
        .all(|(actual, expected)| match (actual, expected) {
            (None, None) => true,
            (Some(actual), Some(expected)) => std::ptr::fn_addr_eq(actual, expected),
            _ => false,
        });
        (parser_matches
            && actual.is_integer == expected.is_integer
            && std::ptr::fn_addr_eq(actual.format, expected.format))
        .then_some(canonical_id)
    }

    /// Requires builtin configured result hooks, including absence of appended hooks.
    fn configuration(&self, type_id: TypeId, canonical_id: TypeId) -> bool {
        match (
            self.registry.type_configuration_descriptor(type_id),
            self.canonical.type_configuration_descriptor(canonical_id),
        ) {
            (None, None) => true,
            (Some(actual), Some(expected)) => {
                let borrowed = match (actual.transform_result, expected.transform_result) {
                    (None, None) => true,
                    (Some(actual), Some(expected)) => std::ptr::fn_addr_eq(actual, expected),
                    _ => false,
                };
                let owned = match (
                    actual.transform_owned_result,
                    expected.transform_owned_result,
                ) {
                    (None, None) => true,
                    (Some(actual), Some(expected)) => std::ptr::fn_addr_eq(actual, expected),
                    _ => false,
                };
                let format = match (actual.format, expected.format) {
                    (None, None) => true,
                    (Some(actual), Some(expected)) => std::ptr::fn_addr_eq(actual, expected),
                    _ => false,
                };
                borrowed
                    && owned
                    && format
                    && actual.initial_result_transform_is_identity
                        == expected.initial_result_transform_is_identity
            }
            _ => false,
        }
    }

    /// Checks the boolean evaluator and configured boolean result identity.
    pub(super) fn callbacks_are_trusted(&self) -> bool {
        let Ok(actual_type) = self.registry.default_boolean() else {
            return false;
        };
        let Some(expected_type) = self.canonical_type(actual_type) else {
            return false;
        };
        if self.canonical.default_boolean().ok() != Some(expected_type) {
            return false;
        }
        match (
            self.registry.default_boolean_evaluator(),
            self.canonical.default_boolean_evaluator(),
        ) {
            (Some(actual), Some(expected)) => {
                std::ptr::fn_addr_eq(actual, expected)
                    && self.configuration(actual_type, expected_type)
            }
            _ => false,
        }
    }

    /// Verifies the resolved operator's executors and complete operand/result types.
    pub(super) fn operator(&self, operator: OperatorId) -> bool {
        self.dispatch_inventory_trusted && self.operator_identity(operator)
    }

    /// Compares one callback without recursively checking overflow dispatch inventory.
    fn operator_identity(&self, operator: OperatorId) -> bool {
        let Ok(actual) = self.registry.operator(operator) else {
            return false;
        };
        let Some(left) = self.canonical_type(actual.left_operand_type) else {
            return false;
        };
        let Some(right) = self.canonical_type(actual.right_operand_type) else {
            return false;
        };
        let Some(result) = self.canonical_type(actual.result_type) else {
            return false;
        };
        let Ok(expected_id) = self
            .canonical
            .resolve_binary_operator(actual.operator, left, right)
        else {
            return false;
        };
        let Ok(expected) = self.canonical.operator(expected_id) else {
            return false;
        };
        let context = match (actual.context_execute, expected.context_execute) {
            (None, None) => true,
            (Some(actual), Some(expected)) => std::ptr::fn_addr_eq(actual, expected),
            _ => false,
        };
        let in_place = match (actual.in_place_execute, expected.in_place_execute) {
            (None, None) => true,
            (Some(actual), Some(expected)) => std::ptr::fn_addr_eq(actual, expected),
            _ => false,
        };
        expected.result_type == result && std::ptr::fn_addr_eq(actual.execute, expected.execute)
            && context && in_place && self.configuration(actual.result_type, result)
            // Context executors may promote or resolve default fractional operators.
            && self.promotion_configuration_is_trusted()
    }

    /// Checks all installed builtin scalar signatures reachable after integer promotion.
    fn dispatch_inventory(&self) -> bool {
        let mut types = Vec::new();
        for (_, canonical_id) in self.canonical.registered_type_entries() {
            let Ok(descriptor) = self.canonical.type_descriptor(canonical_id) else {
                return false;
            };
            if let Some(actual) = self.registry.type_by_name(descriptor.name) {
                if !types.contains(&actual) {
                    types.push(actual);
                }
                if self.canonical_type(actual) != Some(canonical_id)
                    || !self.configuration(actual, canonical_id)
                {
                    return false;
                }
            }
        }
        for &left in &types {
            for &right in &types {
                for operator in [
                    crate::semantic::BinaryOperator::Addition,
                    crate::semantic::BinaryOperator::Subtraction,
                    crate::semantic::BinaryOperator::Multiplication,
                    crate::semantic::BinaryOperator::Division,
                    crate::semantic::BinaryOperator::Remainder,
                    crate::semantic::BinaryOperator::Power,
                ] {
                    if let Ok(operator) =
                        self.registry.resolve_binary_operator(operator, left, right)
                        && !self.operator_identity(operator)
                    {
                        return false;
                    }
                }
                for relation in [
                    crate::semantic::ComparisonOperator::Equal,
                    crate::semantic::ComparisonOperator::NotEqual,
                    crate::semantic::ComparisonOperator::Less,
                    crate::semantic::ComparisonOperator::LessOrEqual,
                    crate::semantic::ComparisonOperator::Greater,
                    crate::semantic::ComparisonOperator::GreaterOrEqual,
                ] {
                    if let Ok(comparison) = self.registry.resolve_comparison(relation, left, right)
                    {
                        let Ok(actual) = self.registry.comparison(comparison) else {
                            return false;
                        };
                        let Ok(expected_id) = self.canonical.resolve_comparison(
                            relation,
                            self.canonical_type(left).unwrap(),
                            self.canonical_type(right).unwrap(),
                        ) else {
                            return false;
                        };
                        let Ok(expected) = self.canonical.comparison(expected_id) else {
                            return false;
                        };
                        if !std::ptr::fn_addr_eq(actual.execute, expected.execute) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Ensures overflow promotions cannot reach appended callbacks on integer types.
    fn promotion_configuration_is_trusted(&self) -> bool {
        if self
            .registry
            .default_integer()
            .ok()
            .and_then(|actual| self.canonical_type(actual))
            != self.canonical.default_integer().ok()
            || self
                .registry
                .default_fractional()
                .ok()
                .and_then(|actual| self.canonical_type(actual))
                != self.canonical.default_fractional().ok()
        {
            return false;
        }
        [
            "int8", "int16", "int32", "int64", "int128", "bigint", "float", "double", "decimal",
        ]
        .into_iter()
        .all(|name| {
            let Some(actual) = self.registry.type_by_name(name) else {
                return true;
            };
            let Some(expected) = self.canonical_type(actual) else {
                return false;
            };
            self.configuration(actual, expected)
        })
    }

    /// Verifies relation callback identities and the configured boolean output.
    pub(super) fn comparison(&self, comparison: ComparisonId) -> bool {
        if !self.dispatch_inventory_trusted {
            return false;
        }
        let Ok(actual) = self.registry.comparison(comparison) else {
            return false;
        };
        let Some(left) = self.canonical_type(actual.left_operand_type) else {
            return false;
        };
        let Some(right) = self.canonical_type(actual.right_operand_type) else {
            return false;
        };
        let Ok(expected_id) = self
            .canonical
            .resolve_comparison(actual.operator, left, right)
        else {
            return false;
        };
        let Ok(expected) = self.canonical.comparison(expected_id) else {
            return false;
        };
        std::ptr::fn_addr_eq(actual.execute, expected.execute) && self.callbacks_are_trusted()
    }

    /// Requires lossless builtin signed arithmetic; unsigned saturation is not affine.
    pub(super) fn signed_integer(&self, value_type: ValueType) -> bool {
        if value_type.subtype.is_some() {
            return false;
        }
        let Some(canonical) = self.canonical_type(value_type.base) else {
            return false;
        };
        ["int8", "int16", "int32", "int64", "int128", "bigint"]
            .into_iter()
            .any(|name| self.canonical.type_by_name(name) == Some(canonical))
            && self.configuration(value_type.base, canonical)
    }

    /// Verifies an embedded index callback, including custom replacement extractors.
    pub(super) fn index_extractor(&self, type_id: TypeId, extractor: IndexExtractor) -> bool {
        let Some(canonical) = self.canonical_type(type_id) else {
            return false;
        };
        self.canonical
            .index_extractor(canonical)
            .is_some_and(|expected| std::ptr::fn_addr_eq(extractor, expected))
    }

    /// Reads small signed builtin literals without invoking registry callbacks.
    pub(super) fn integer_literal(&self, value: &Value) -> Option<i128> {
        if !self.signed_integer(value.value_type()) {
            return None;
        }
        value
            .downcast_ref::<i8>()
            .map(|value| i128::from(*value))
            .or_else(|| value.downcast_ref::<i16>().map(|value| i128::from(*value)))
            .or_else(|| value.downcast_ref::<i32>().map(|value| i128::from(*value)))
            .or_else(|| value.downcast_ref::<i64>().map(|value| i128::from(*value)))
            .or_else(|| value.downcast_ref::<i128>().copied())
    }

    /// Proves formatter/parser conversions for a builtin element representation.
    pub(super) fn element_store(
        &self,
        element: ValueType,
        expression: &crate::ir::TypedExpression,
    ) -> bool {
        if !self.dispatch_inventory_trusted {
            return false;
        }
        let Some(target) = self.canonical_type(element.base) else {
            return false;
        };
        if !self.configuration(element.base, target) {
            return false;
        }
        if let Some(domain) = expression.complete_type_domain() {
            return domain.candidates.iter().all(|value_type| {
                self.canonical_type(value_type.base)
                    .is_some_and(|canonical| self.configuration(value_type.base, canonical))
            }) && self.promotion_configuration_is_trusted();
        }
        match &expression.output {
            Some(crate::semantic::SemanticType::Scalar(value_type)) => {
                self.canonical_type(value_type.base)
                    .is_some_and(|canonical| self.configuration(value_type.base, canonical))
                    && self.promotion_configuration_is_trusted()
            }
            _ => false,
        }
    }

    /// Guards builtin native calls that parse results using the registry's default string.
    pub(super) fn function_callbacks(&self, function: crate::semantic::FunctionId) -> bool {
        let Ok(actual) = self.registry.function(function) else {
            return false;
        };
        let signature = match &actual.signature {
            crate::semantic::FunctionSignature::Exact(types) => {
                let Some(types) = types
                    .iter()
                    .map(|type_id| self.canonical_type(*type_id))
                    .collect::<Option<Vec<_>>>()
                else {
                    // Explicit extension purity is its trusted contract.
                    return true;
                };
                types
            }
            _ => return true,
        };
        let Ok(expected_id) = self.canonical.resolve_function(actual.name, &signature) else {
            return true;
        };
        let Ok(expected) = self.canonical.function(expected_id) else {
            return false;
        };
        if !std::ptr::fn_addr_eq(actual.execute, expected.execute) {
            return true;
        }
        let Ok(actual_string) = self.registry.default_string() else {
            return false;
        };
        let Some(expected_string) = self.canonical_type(actual_string) else {
            return false;
        };
        self.canonical.default_string().ok() == Some(expected_string)
            && self.configuration(actual_string, expected_string)
    }
}

#[cfg(test)]
#[path = "builtins.tests.rs"]
mod tests;
