use ir::{
    BindingId, LocalVariableSlot, TypedBinaryExecutionPlan, TypedBlock, TypedExpression,
    TypedExpressionKind, TypedProgram, TypedRangePlan, TypedScalePlan, TypedScaleStep,
    TypedStatement,
};
use language_core::{
    BinaryOperator, CoreError, Registry, Scale, SubtypeBinaryRule, SubtypeDescriptor,
    SubtypeRelativeRule, TypeDescriptor, Value, ValueType,
};
use syntax::Span;

use super::*;

const SPAN: Span = Span { start: 0, end: 0 };

fn parse_integer(raw_text: &str, type_id: language_core::TypeId) -> Result<Value, CoreError> {
    let value = raw_text
        .parse::<i64>()
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.to_string(),
            type_name: "int".to_string(),
            message: error.to_string(),
        })?;
    Ok(Value::new(type_id, value))
}

fn parse_fractional(raw_text: &str, type_id: language_core::TypeId) -> Result<Value, CoreError> {
    let value = raw_text
        .parse::<f64>()
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.to_string(),
            type_name: "fractional".to_string(),
            message: error.to_string(),
        })?;
    Ok(Value::new(type_id, value))
}

fn format_value(_: &Value) -> Result<String, CoreError> {
    Ok(String::new())
}

fn divide_integer_by_fractional(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let integer = left_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    let fractional = right_operand
        .downcast_ref::<f64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("fractional".to_string()))?;
    Ok(Value::new(
        right_operand.type_id(),
        *integer as f64 / fractional,
    ))
}

fn multiply_integers(left_operand: &Value, right_operand: &Value) -> Result<Value, CoreError> {
    let left_integer = left_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    let right_integer = right_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    Ok(Value::new(
        left_operand.type_id(),
        left_integer * right_integer,
    ))
}

fn multiply_integer_by_fractional(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let integer = left_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    let fractional = right_operand
        .downcast_ref::<f64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("fractional".to_string()))?;
    Ok(Value::new(
        right_operand.type_id(),
        *integer as f64 * fractional,
    ))
}

/// Subtracts a fractional subtrahend from an integer minuend for fixtures.
///
/// Tests use this as the outer operator of a relative subtraction, where the
/// left magnitude stays an integer while its percentage adjustment already
/// promoted to the fractional representation.
fn subtract_fractional_from_integer(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let integer = left_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    let fractional = right_operand
        .downcast_ref::<f64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("fractional".to_string()))?;
    Ok(Value::new(
        right_operand.type_id(),
        *integer as f64 - fractional,
    ))
}

fn evaluate_boolean(value: &Value) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bool".into()))
}

fn conditional_registry() -> (Registry, language_core::TypeId) {
    let mut registry = Registry::new();
    let boolean = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: boolean,
            name: "bool",
            is_integer: false,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry
        .set_default_boolean(boolean, evaluate_boolean)
        .unwrap();
    (registry, boolean)
}

fn boolean_expression(boolean_type: language_core::TypeId, value: bool) -> TypedExpression {
    TypedExpression {
        output: Some(ValueType::plain(boolean_type)),
        kind: TypedExpressionKind::Literal(Value::new(boolean_type, value)),
        span: SPAN,
    }
}

fn missing_variable_expression(boolean_type: language_core::TypeId) -> TypedExpression {
    TypedExpression {
        output: Some(ValueType::plain(boolean_type)),
        kind: TypedExpressionKind::Variable {
            name: "missing".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            nullable: false,
        },
        span: SPAN,
    }
}

fn registry() -> (
    Registry,
    language_core::TypeId,
    language_core::TypeId,
    language_core::SubtypeId,
) {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let fractional = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: integer,
            name: "int",
            is_integer: true,
            parse_numeric_literal: Some(parse_integer),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry
        .register_type(TypeDescriptor {
            id: fractional,
            name: "fractional",
            is_integer: false,
            parse_numeric_literal: Some(parse_fractional),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry.set_default_integer(integer).unwrap();
    registry.set_default_fractional(fractional).unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Division,
            integer,
            fractional,
            fractional,
            divide_integer_by_fractional,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Multiplication,
            integer,
            integer,
            integer,
            multiply_integers,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Multiplication,
            integer,
            fractional,
            fractional,
            multiply_integer_by_fractional,
        )
        .unwrap();
    let percentage = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id: percentage,
            name: "percentage",
            suffixes: &["%"],
        })
        .unwrap();
    registry
        .register_subtype_binary_rule(
            BinaryOperator::Multiplication,
            None,
            Some(percentage),
            SubtypeBinaryRule::new(None).with_operand_scales(Scale::IDENTITY, Scale::new(1, 100)),
        )
        .unwrap();

    (registry, integer, fractional, percentage)
}

#[test]
fn fractional_subtype_scale_executes_with_the_promoted_operator() {
    let (registry, integer, fractional, percentage) = registry();
    let resolution = registry
        .resolve_binary_operation(
            BinaryOperator::Multiplication,
            ValueType::plain(integer),
            ValueType::qualified(integer, percentage),
        )
        .unwrap();
    assert_eq!(resolution.output, ValueType::plain(fractional));
    let scale_operator = registry
        .resolve_binary_operator(BinaryOperator::Division, integer, fractional)
        .unwrap();
    let execution_plan = TypedBinaryExecutionPlan {
        left_operand_scale: TypedScalePlan::default(),
        right_operand_scale: TypedScalePlan {
            numerator: None,
            denominator: Some(TypedScaleStep {
                operator: scale_operator,
                factor: registry.parse_numeric("100", Some(fractional)).unwrap(),
            }),
        },
        relative_adjustment_operator: None,
    };

    let program = TypedProgram {
        statements: vec![TypedStatement::Expression(TypedExpression {
            output: Some(resolution.output),
            kind: TypedExpressionKind::Binary {
                resolution,
                execution_plan: Box::new(execution_plan),
                left_operand: Box::new(TypedExpression {
                    output: Some(ValueType::plain(integer)),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 100_i64)),
                    span: SPAN,
                }),
                right_operand: Box::new(TypedExpression {
                    output: Some(ValueType::qualified(integer, percentage)),
                    kind: TypedExpressionKind::Literal(
                        Value::new(integer, 10_i64).with_subtype(Some(percentage)),
                    ),
                    span: SPAN,
                }),
            },
            span: SPAN,
        })],
        local_slot_count: 0,
        bindings: Vec::new(),
    };

    execute(&program, &registry).unwrap();
}

#[test]
fn relative_subtype_subtraction_combines_the_left_magnitude_with_its_fraction() {
    let (mut registry, integer, fractional, percentage) = registry();
    registry
        .register_binary_operator(
            BinaryOperator::Subtraction,
            integer,
            fractional,
            fractional,
            subtract_fractional_from_integer,
        )
        .unwrap();
    registry
        .register_subtype_relative_rule(
            BinaryOperator::Subtraction,
            None,
            Some(percentage),
            SubtypeRelativeRule::new(Scale::new(1, 100)),
        )
        .unwrap();
    let resolution = registry
        .resolve_subtype_relative_rule(
            BinaryOperator::Subtraction,
            ValueType::plain(integer),
            ValueType::qualified(integer, percentage),
        )
        .unwrap();
    assert_eq!(resolution.output, ValueType::plain(fractional));
    let scale_operator = registry
        .resolve_binary_operator(BinaryOperator::Division, integer, fractional)
        .unwrap();
    let adjustment_operator = registry
        .resolve_binary_operator(BinaryOperator::Multiplication, integer, fractional)
        .unwrap();
    let execution_plan = TypedBinaryExecutionPlan {
        left_operand_scale: TypedScalePlan::default(),
        right_operand_scale: TypedScalePlan {
            numerator: None,
            denominator: Some(TypedScaleStep {
                operator: scale_operator,
                factor: registry.parse_numeric("100", Some(fractional)).unwrap(),
            }),
        },
        relative_adjustment_operator: Some(adjustment_operator),
    };

    let program = TypedProgram {
        statements: vec![TypedStatement::Expression(TypedExpression {
            output: Some(resolution.output),
            kind: TypedExpressionKind::Binary {
                resolution,
                execution_plan: Box::new(execution_plan),
                left_operand: Box::new(TypedExpression {
                    output: Some(ValueType::plain(integer)),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 100_i64)),
                    span: SPAN,
                }),
                right_operand: Box::new(TypedExpression {
                    output: Some(ValueType::qualified(integer, percentage)),
                    kind: TypedExpressionKind::Literal(
                        Value::new(integer, 50_i64).with_subtype(Some(percentage)),
                    ),
                    span: SPAN,
                }),
            },
            span: SPAN,
        })],
        local_slot_count: 0,
        bindings: Vec::new(),
    };

    execute(&program, &registry).unwrap();
}

#[test]
fn false_if_condition_skips_its_body() {
    let (registry, boolean) = conditional_registry();
    let program = TypedProgram {
        statements: vec![TypedStatement::If {
            condition: boolean_expression(boolean, false),
            body: TypedBlock {
                statements: vec![TypedStatement::Expression(missing_variable_expression(
                    boolean,
                ))],
                span: SPAN,
            },
            else_body: None,
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    execute(&program, &registry).unwrap();
}

/// Verifies logical expressions avoid evaluating a right operand when short-circuiting decides the result.
#[test]
fn logical_operations_short_circuit_runtime_evaluation() {
    let (registry, boolean) = conditional_registry();
    let missing = missing_variable_expression(boolean);
    let program = TypedProgram {
        statements: vec![
            TypedStatement::Expression(TypedExpression {
                output: Some(ValueType::plain(boolean)),
                kind: TypedExpressionKind::Logical {
                    operator: syntax::LogicalOperator::And,
                    left_operand: Box::new(boolean_expression(boolean, false)),
                    right_operand: Box::new(missing.clone()),
                },
                span: SPAN,
            }),
            TypedStatement::Expression(TypedExpression {
                output: Some(ValueType::plain(boolean)),
                kind: TypedExpressionKind::Logical {
                    operator: syntax::LogicalOperator::Or,
                    left_operand: Box::new(boolean_expression(boolean, true)),
                    right_operand: Box::new(missing),
                },
                span: SPAN,
            }),
        ],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    execute(&program, &registry).unwrap();
}

#[test]
fn true_if_condition_executes_its_body_and_releases_local_variables() {
    let (registry, boolean) = conditional_registry();
    let program = TypedProgram {
        statements: vec![
            TypedStatement::If {
                condition: boolean_expression(boolean, true),
                body: TypedBlock {
                    statements: vec![TypedStatement::VariableDeclaration {
                        name: "local".into(),
                        binding: BindingId(0),
                        slot: LocalVariableSlot(0),
                        mutable: false,
                        value_type: ValueType::plain(boolean),
                        expression: boolean_expression(boolean, true),
                        span: SPAN,
                    }],
                    span: SPAN,
                },
                else_body: None,
                span: SPAN,
            },
            TypedStatement::Expression(TypedExpression {
                output: Some(ValueType::plain(boolean)),
                kind: TypedExpressionKind::Variable {
                    name: "local".into(),
                    binding: BindingId(1),
                    slot: LocalVariableSlot(1),
                    nullable: false,
                },
                span: SPAN,
            }),
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let error = execute(&program, &registry).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unknown runtime variable `local`")
    );
}

fn less_integers(left_operand: &Value, right_operand: &Value) -> Result<bool, CoreError> {
    let left_integer = left_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    let right_integer = right_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    Ok(left_integer < right_integer)
}

fn add_integers(left_operand: &Value, right_operand: &Value) -> Result<Value, CoreError> {
    let left_integer = left_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    let right_integer = right_operand
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".to_string()))?;
    left_integer
        .checked_add(*right_integer)
        .map(|sum| Value::new(left_operand.type_id(), sum))
        .ok_or_else(|| CoreError::Runtime("integer overflow in addition".into()))
}

fn integer_range_registry() -> (Registry, language_core::TypeId) {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let boolean = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: integer,
            name: "int",
            is_integer: true,
            parse_numeric_literal: Some(parse_integer),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry
        .register_type(TypeDescriptor {
            id: boolean,
            name: "bool",
            is_integer: false,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry.set_default_integer(integer).unwrap();
    registry
        .set_default_boolean(boolean, evaluate_boolean)
        .unwrap();
    registry
        .register_comparison(
            language_core::ComparisonOperator::Less,
            integer,
            integer,
            less_integers,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Addition,
            integer,
            integer,
            integer,
            add_integers,
        )
        .unwrap();
    (registry, integer)
}

fn integer_literal(integer: language_core::TypeId, value: i64) -> TypedExpression {
    TypedExpression {
        output: Some(ValueType::plain(integer)),
        kind: TypedExpressionKind::Literal(Value::new(integer, value)),
        span: SPAN,
    }
}

fn integer_variable(
    integer: language_core::TypeId,
    name: &str,
    slot: LocalVariableSlot,
) -> TypedExpression {
    TypedExpression {
        output: Some(ValueType::plain(integer)),
        kind: TypedExpressionKind::Variable {
            name: name.into(),
            binding: BindingId(slot.0),
            slot,
            nullable: false,
        },
        span: SPAN,
    }
}

/// Builds the compiler-equivalent cached plan for a direct runtime range test.
fn integer_range_plan(registry: &Registry, integer: language_core::TypeId) -> TypedRangePlan {
    let current_type = ValueType::plain(integer);
    let increment_unit = registry.parse_numeric("1", Some(integer)).unwrap();
    TypedRangePlan {
        current_type,
        comparison: registry
            .resolve_comparison_operation(
                language_core::ComparisonOperator::Less,
                current_type,
                current_type,
            )
            .unwrap(),
        increment: registry
            .resolve_binary_operation(
                BinaryOperator::Addition,
                current_type,
                increment_unit.value_type(),
            )
            .unwrap(),
        increment_unit,
    }
}

#[test]
fn for_range_binds_each_value_and_releases_its_scope() {
    let (registry, integer) = integer_range_registry();
    let program = TypedProgram {
        statements: vec![
            TypedStatement::For {
                variable: "index".into(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                variable_type: ValueType::plain(integer),
                start: integer_literal(integer, 0),
                end: integer_literal(integer, 3),
                range_plan: integer_range_plan(&registry, integer),
                body: TypedBlock {
                    statements: vec![TypedStatement::Expression(integer_variable(
                        integer,
                        "index",
                        LocalVariableSlot(0),
                    ))],
                    span: SPAN,
                },
                span: SPAN,
            },
            TypedStatement::Expression(integer_variable(integer, "index", LocalVariableSlot(1))),
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let error = execute(&program, &registry).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unknown runtime variable `index`")
    );
}

#[test]
fn for_range_with_empty_bounds_skips_its_body() {
    let (registry, integer) = integer_range_registry();
    let program = TypedProgram {
        statements: vec![TypedStatement::For {
            variable: "index".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            variable_type: ValueType::plain(integer),
            start: integer_literal(integer, 5),
            end: integer_literal(integer, 5),
            range_plan: integer_range_plan(&registry, integer),
            body: TypedBlock {
                statements: vec![TypedStatement::Expression(missing_variable_expression(
                    integer,
                ))],
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    execute(&program, &registry).unwrap();
}

#[test]
fn for_range_with_reversed_bounds_skips_its_body() {
    let (registry, integer) = integer_range_registry();
    let program = TypedProgram {
        statements: vec![TypedStatement::For {
            variable: "index".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            variable_type: ValueType::plain(integer),
            start: integer_literal(integer, 10),
            end: integer_literal(integer, 5),
            range_plan: integer_range_plan(&registry, integer),
            body: TypedBlock {
                statements: vec![TypedStatement::Expression(missing_variable_expression(
                    integer,
                ))],
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    execute(&program, &registry).unwrap();
}

#[test]
fn for_range_rejects_non_integer_bounds() {
    let (mut registry, integer) = integer_range_registry();
    let fractional = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: fractional,
            name: "fractional",
            is_integer: false,
            parse_numeric_literal: Some(parse_fractional),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    let program = TypedProgram {
        statements: vec![TypedStatement::For {
            variable: "index".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            variable_type: ValueType::plain(integer),
            start: integer_literal(integer, 0),
            end: TypedExpression {
                output: Some(ValueType::plain(fractional)),
                kind: TypedExpressionKind::Literal(Value::new(fractional, 1.5_f64)),
                span: SPAN,
            },
            range_plan: integer_range_plan(&registry, integer),
            body: TypedBlock {
                statements: Vec::new(),
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    let error = execute(&program, &registry).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("for range bounds must be integers")
    );
}
