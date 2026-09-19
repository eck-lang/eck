use crate::containers::array::ArrayValue;
use crate::ir::{
    ArrayMethod, BindingId, LocalVariableSlot, TypedBinaryExecutionPlan, TypedBlock,
    TypedExpression, TypedExpressionKind, TypedProgram, TypedRangePlan, TypedScalePlan,
    TypedScaleStep, TypedStatement,
};
use crate::semantic::{
    ArrayElementMode, ArrayType, BinaryOperator, CoreError, Registry, ResolvedSubtypeConversion,
    Scale, SemanticType, SubtypeBinaryRule, SubtypeDescriptor, SubtypeRelativeRule, TypeDescriptor,
    Value, ValueType,
};
use crate::syntax::Span;

use super::*;

const SPAN: Span = Span { start: 0, end: 0 };

fn parse_integer(raw_text: &str, type_id: crate::semantic::TypeId) -> Result<Value, CoreError> {
    let value = raw_text
        .parse::<i64>()
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.to_string(),
            type_name: "int".to_string(),
            message: error.to_string(),
        })?;
    Ok(Value::new(type_id, value))
}

fn parse_fractional(raw_text: &str, type_id: crate::semantic::TypeId) -> Result<Value, CoreError> {
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

fn conditional_registry() -> (Registry, crate::semantic::TypeId) {
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

fn boolean_expression(boolean_type: crate::semantic::TypeId, value: bool) -> TypedExpression {
    TypedExpression {
        output: Some(SemanticType::Scalar(ValueType::plain(boolean_type))),
        kind: TypedExpressionKind::Literal(Value::new(boolean_type, value)),
        span: SPAN,
    }
}

fn missing_variable_expression(boolean_type: crate::semantic::TypeId) -> TypedExpression {
    TypedExpression {
        output: Some(SemanticType::Scalar(ValueType::plain(boolean_type))),
        kind: TypedExpressionKind::Variable {
            name: "missing".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            nullable: false,
            complete_type_domain: None,
        },
        span: SPAN,
    }
}

fn registry() -> (
    Registry,
    crate::semantic::TypeId,
    crate::semantic::TypeId,
    crate::semantic::SubtypeId,
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
        result_domain: None,
    };

    let program = TypedProgram {
        statements: vec![TypedStatement::Expression(TypedExpression {
            output: Some(SemanticType::Scalar(resolution.output)),
            kind: TypedExpressionKind::Binary {
                resolution,
                execution_plan: Box::new(execution_plan),
                left_operand: Box::new(TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::plain(integer))),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 100_i64)),
                    span: SPAN,
                }),
                right_operand: Box::new(TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::qualified(
                        integer, percentage,
                    ))),
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
        result_domain: None,
    };

    let program = TypedProgram {
        statements: vec![TypedStatement::Expression(TypedExpression {
            output: Some(SemanticType::Scalar(resolution.output)),
            kind: TypedExpressionKind::Binary {
                resolution,
                execution_plan: Box::new(execution_plan),
                left_operand: Box::new(TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::plain(integer))),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 100_i64)),
                    span: SPAN,
                }),
                right_operand: Box::new(TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::qualified(
                        integer, percentage,
                    ))),
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
                owned_slots: Box::new([]),
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
                output: Some(SemanticType::Scalar(ValueType::plain(boolean))),
                kind: TypedExpressionKind::Logical {
                    operator: crate::syntax::LogicalOperator::And,
                    left_operand: Box::new(boolean_expression(boolean, false)),
                    right_operand: Box::new(missing.clone()),
                },
                span: SPAN,
            }),
            TypedStatement::Expression(TypedExpression {
                output: Some(SemanticType::Scalar(ValueType::plain(boolean))),
                kind: TypedExpressionKind::Logical {
                    operator: crate::syntax::LogicalOperator::Or,
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
                        semantic_type: SemanticType::Scalar(ValueType::plain(boolean)),
                        expression: boolean_expression(boolean, true),
                        span: SPAN,
                    }],
                    owned_slots: Box::new([LocalVariableSlot(0)]),
                    span: SPAN,
                },
                else_body: None,
                span: SPAN,
            },
            TypedStatement::Expression(TypedExpression {
                output: Some(SemanticType::Scalar(ValueType::plain(boolean))),
                kind: TypedExpressionKind::Variable {
                    name: "local".into(),
                    binding: BindingId(1),
                    slot: LocalVariableSlot(1),
                    nullable: false,
                    complete_type_domain: None,
                },
                span: SPAN,
            }),
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);
    let error = result.unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unknown runtime variable `local`")
    );
    assert!(locals[0].is_none());
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

fn integer_range_registry() -> (Registry, crate::semantic::TypeId) {
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
            crate::semantic::ComparisonOperator::Less,
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

fn integer_literal(integer: crate::semantic::TypeId, value: i64) -> TypedExpression {
    TypedExpression {
        output: Some(SemanticType::Scalar(ValueType::plain(integer))),
        kind: TypedExpressionKind::Literal(Value::new(integer, value)),
        span: SPAN,
    }
}

/// Builds one scalar declaration for local-slot cleanup tests.
fn integer_declaration(
    slot: usize,
    integer: crate::semantic::TypeId,
    value: i64,
) -> TypedStatement {
    TypedStatement::VariableDeclaration {
        name: format!("value{slot}"),
        binding: BindingId(slot),
        slot: LocalVariableSlot(slot),
        mutable: false,
        semantic_type: SemanticType::Scalar(ValueType::plain(integer)),
        expression: integer_literal(integer, value),
        span: SPAN,
    }
}

fn integer_variable(
    integer: crate::semantic::TypeId,
    name: &str,
    slot: LocalVariableSlot,
) -> TypedExpression {
    TypedExpression {
        output: Some(SemanticType::Scalar(ValueType::plain(integer))),
        kind: TypedExpressionKind::Variable {
            name: name.into(),
            binding: BindingId(slot.0),
            slot,
            nullable: false,
            complete_type_domain: None,
        },
        span: SPAN,
    }
}

/// Builds the compiler-equivalent cached plan for a direct runtime range test.
fn integer_range_plan(registry: &Registry, integer: crate::semantic::TypeId) -> TypedRangePlan {
    let current_type = ValueType::plain(integer);
    let increment_unit = registry.parse_numeric("1", Some(integer)).unwrap();
    TypedRangePlan {
        current_type,
        comparison: registry
            .resolve_comparison_operation(
                crate::semantic::ComparisonOperator::Less,
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
        statements: vec![TypedStatement::For {
            variable: "index".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            variable_type: ValueType::plain(integer),
            start: integer_literal(integer, 0),
            end: integer_literal(integer, 3),
            range_plan: integer_range_plan(&registry, integer),
            body: TypedBlock {
                statements: vec![
                    array_declaration(1, integer, vec![1]),
                    TypedStatement::Expression(integer_variable(
                        integer,
                        "index",
                        LocalVariableSlot(0),
                    )),
                ],
                owned_slots: Box::new([LocalVariableSlot(1)]),
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert!(locals.iter().all(Option::is_none));
}

/// Verifies direct block slots are cleared after normal execution and errors.
#[test]
fn blocks_clear_array_and_value_slots_on_success_and_error() {
    let (registry, integer) = integer_range_registry();
    let successful = TypedProgram {
        statements: vec![TypedStatement::Block(TypedBlock {
            statements: vec![
                array_declaration(0, integer, vec![1]),
                integer_declaration(1, integer, 2),
            ],
            owned_slots: Box::new([LocalVariableSlot(0), LocalVariableSlot(1)]),
            span: SPAN,
        })],
        local_slot_count: 2,
        bindings: Vec::new(),
    };
    let (result, locals) = execute_collecting_locals(&successful, &registry);
    result.unwrap();
    assert!(locals.iter().all(Option::is_none));

    let failing = TypedProgram {
        statements: vec![TypedStatement::Block(TypedBlock {
            statements: vec![
                array_declaration(0, integer, vec![1]),
                TypedStatement::Expression(integer_variable(
                    integer,
                    "missing",
                    LocalVariableSlot(1),
                )),
            ],
            owned_slots: Box::new([LocalVariableSlot(0)]),
            span: SPAN,
        })],
        local_slot_count: 2,
        bindings: Vec::new(),
    };
    let (result, locals) = execute_collecting_locals(&failing, &registry);
    assert!(result.is_err());
    assert!(locals[0].is_none());
}

/// Verifies direct loop-body slots are cleared on each early control transfer.
#[test]
fn optimized_for_body_clears_owned_slots_on_break_and_continue() {
    let (registry, integer) = integer_range_registry();
    for control_statement in [
        TypedStatement::Break { span: SPAN },
        TypedStatement::Continue { span: SPAN },
    ] {
        let program = TypedProgram {
            statements: vec![TypedStatement::For {
                variable: "index".into(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                variable_type: ValueType::plain(integer),
                start: integer_literal(integer, 0),
                end: integer_literal(integer, 1),
                range_plan: integer_range_plan(&registry, integer),
                body: TypedBlock {
                    statements: vec![
                        array_declaration(1, integer, vec![1]),
                        control_statement.clone(),
                    ],
                    owned_slots: Box::new([LocalVariableSlot(1)]),
                    span: SPAN,
                },
                span: SPAN,
            }],
            local_slot_count: 2,
            bindings: Vec::new(),
        };
        let (result, locals) = execute_collecting_locals(&program, &registry);
        result.unwrap();
        assert!(locals.iter().all(Option::is_none));
    }
}

/// Verifies releasing an inner array alias leaves the outer value live for COW.
#[test]
fn clearing_an_inner_array_alias_preserves_the_outer_value() {
    let (registry, integer, _) = array_method_registry();
    let element_type = ValueType::plain(integer);
    let array_type = ArrayType {
        element: element_type,
        element_mode: ArrayElementMode::Exact,
    };
    let alias = TypedStatement::VariableDeclaration {
        name: "alias".into(),
        binding: BindingId(1),
        slot: LocalVariableSlot(1),
        mutable: true,
        semantic_type: SemanticType::Array(array_type),
        expression: TypedExpression {
            output: Some(SemanticType::Array(array_type)),
            kind: TypedExpressionKind::Variable {
                name: "values0".into(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                nullable: false,
                complete_type_domain: None,
            },
            span: SPAN,
        },
        span: SPAN,
    };
    let update = TypedStatement::IndexedAssignment {
        name: "alias".into(),
        binding: BindingId(1),
        slot: LocalVariableSlot(1),
        index: integer_literal(integer, 0),
        constant_index: Some(0),
        index_extractor: extract_integer_index,
        index_dispatch: None,
        expression: TypedExpression {
            output: Some(SemanticType::Scalar(element_type)),
            kind: TypedExpressionKind::Literal(Value::new(integer, 9_i64)),
            span: SPAN,
        },
        span: SPAN,
    };
    let program = TypedProgram {
        statements: vec![
            array_declaration(0, integer, vec![1, 2]),
            TypedStatement::Block(TypedBlock {
                statements: vec![alias, update],
                owned_slots: Box::new([LocalVariableSlot(1)]),
                span: SPAN,
            }),
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert_eq!(
        stored_elements(&locals[0])[0].downcast_ref::<i64>(),
        Some(&1)
    );
    assert!(locals[1].is_none());
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
                owned_slots: Box::new([]),
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert!(locals[0].is_none());
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
                owned_slots: Box::new([]),
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert!(locals[0].is_none());
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
                output: Some(SemanticType::Scalar(ValueType::plain(fractional))),
                kind: TypedExpressionKind::Literal(Value::new(fractional, 1.5_f64)),
                span: SPAN,
            },
            range_plan: integer_range_plan(&registry, integer),
            body: TypedBlock {
                statements: Vec::new(),
                owned_slots: Box::new([]),
                span: SPAN,
            },
            span: SPAN,
        }],
        local_slot_count: 1,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);
    let error = result.unwrap_err();

    assert!(
        error
            .to_string()
            .contains("for range bounds must be integers")
    );
    assert!(locals[0].is_none());
}

/// Builds a registry with a narrow and a wide integer representation.
///
/// The array store tests need types whose formatters and parsers round-trip a
/// magnitude, because the store boundary normalizes a widened value by
/// formatting it and re-parsing it as the declared representation. The narrow
/// type also owns the index contract the store statements use.
fn narrow_integer_registry() -> (
    Registry,
    crate::semantic::TypeId,
    crate::semantic::TypeId,
    crate::semantic::SubtypeId,
) {
    let mut registry = Registry::new();
    let narrow = registry.allocate_type_id();
    let wide = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: narrow,
            name: "narrow",
            is_integer: true,
            parse_numeric_literal: Some(parse_narrow_integer),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_narrow_integer,
        })
        .unwrap();
    registry
        .register_type(TypeDescriptor {
            id: wide,
            name: "wide",
            is_integer: true,
            parse_numeric_literal: Some(parse_wide_integer),
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: None,
            format: format_wide_integer,
        })
        .unwrap();
    let millimeter = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id: millimeter,
            name: "millimeter",
            suffixes: &["mm"],
        })
        .unwrap();
    (registry, narrow, wide, millimeter)
}

/// Parses one narrow magnitude, rejecting what the representation cannot hold.
fn parse_narrow_integer(
    raw_text: &str,
    type_id: crate::semantic::TypeId,
) -> Result<Value, CoreError> {
    raw_text
        .parse::<i8>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.to_string(),
            type_name: "narrow".to_string(),
            message: error.to_string(),
        })
}

/// Parses one wide magnitude.
fn parse_wide_integer(
    raw_text: &str,
    type_id: crate::semantic::TypeId,
) -> Result<Value, CoreError> {
    raw_text
        .parse::<i16>()
        .map(|value| Value::new(type_id, value))
        .map_err(|error| CoreError::InvalidLiteral {
            raw_text: raw_text.to_string(),
            type_name: "wide".to_string(),
            message: error.to_string(),
        })
}

/// Formats one narrow magnitude.
fn format_narrow_integer(value: &Value) -> Result<String, CoreError> {
    value
        .downcast_ref::<i8>()
        .map(i8::to_string)
        .ok_or_else(|| CoreError::InvalidValueRepresentation("narrow".to_string()))
}

/// Formats one wide magnitude.
fn format_wide_integer(value: &Value) -> Result<String, CoreError> {
    value
        .downcast_ref::<i16>()
        .map(i16::to_string)
        .ok_or_else(|| CoreError::InvalidValueRepresentation("wide".to_string()))
}

/// Divides two narrow integers with the truncating semantics a subtype scale
/// must reject before it reaches.
fn divide_narrow_integers(left_operand: &Value, right_operand: &Value) -> Result<Value, CoreError> {
    let left = left_operand
        .downcast_ref::<i8>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("narrow".to_string()))?;
    let right = right_operand
        .downcast_ref::<i8>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("narrow".to_string()))?;
    Ok(Value::new(left_operand.type_id(), *left / *right))
}

/// Returns the remainder of two narrow integers for exact scale validation.
fn remainder_narrow_integers(
    left_operand: &Value,
    right_operand: &Value,
) -> Result<Value, CoreError> {
    let left = left_operand
        .downcast_ref::<i8>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("narrow".to_string()))?;
    let right = right_operand
        .downcast_ref::<i8>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("narrow".to_string()))?;
    Ok(Value::new(left_operand.type_id(), *left % *right))
}

/// Extends the narrow fixture with an integer-only millimeter-to-centimeter
/// conversion, so runtime tests can prove the division is checked first.
fn exact_unit_conversion_registry() -> (
    Registry,
    crate::semantic::TypeId,
    crate::semantic::SubtypeId,
    crate::semantic::SubtypeId,
) {
    let (mut registry, narrow, wide, millimeter) = narrow_integer_registry();
    registry.set_default_integer(wide).unwrap();
    let centimeter = registry.allocate_subtype_id();
    registry
        .register_subtype(SubtypeDescriptor {
            id: centimeter,
            name: "centimeter",
            suffixes: &["cm"],
        })
        .unwrap();
    registry
        .register_subtype_conversion(millimeter, centimeter, Scale::new(1, 10))
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Division,
            narrow,
            narrow,
            narrow,
            divide_narrow_integers,
        )
        .unwrap();
    registry
        .register_binary_operator(
            BinaryOperator::Remainder,
            narrow,
            narrow,
            narrow,
            remainder_narrow_integers,
        )
        .unwrap();
    (registry, narrow, millimeter, centimeter)
}

/// Converts one narrow magnitude into a zero-based array index.
fn extract_narrow_index(value: &Value) -> Result<usize, CoreError> {
    let index = value
        .downcast_ref::<i8>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("narrow".to_string()))?;
    usize::try_from(index).map_err(|_| CoreError::Runtime(format!("index {index} is negative")))
}

/// Builds a program that declares `narrow[]` and stores `stored` as element 0.
///
/// The element expression is a literal whose declared output is `narrow` while
/// the value it produces uses the wide representation. That is exactly the state
/// an integer expression leaves behind after it promoted while it was evaluated,
/// so the store boundary has to normalize or reject the value on its own.
fn store_boundary_program(
    narrow: crate::semantic::TypeId,
    element: ValueType,
    stored: Value,
) -> TypedProgram {
    let initial_element = Value::new(narrow, 127_i8).with_subtype(element.subtype);
    TypedProgram {
        statements: vec![
            TypedStatement::VariableDeclaration {
                name: "values".to_string(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                mutable: true,
                semantic_type: SemanticType::Array(ArrayType {
                    element: ValueType::plain(narrow),
                    element_mode: ArrayElementMode::Exact,
                }),
                expression: TypedExpression {
                    output: Some(SemanticType::Array(ArrayType {
                        element: ValueType::plain(narrow),
                        element_mode: ArrayElementMode::Exact,
                    })),
                    kind: TypedExpressionKind::ArrayLiteral {
                        elements: vec![TypedExpression {
                            output: Some(SemanticType::Scalar(ValueType::plain(narrow))),
                            kind: TypedExpressionKind::Literal(initial_element),
                            span: SPAN,
                        }],
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
            TypedStatement::IndexedAssignment {
                name: "values".to_string(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                index: TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::plain(narrow))),
                    kind: TypedExpressionKind::Literal(Value::new(narrow, 0_i8)),
                    span: SPAN,
                },
                constant_index: Some(0),
                index_extractor: extract_narrow_index,
                index_dispatch: None,
                expression: TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::plain(narrow))),
                    kind: TypedExpressionKind::ElementStore {
                        element,
                        expression: Box::new(TypedExpression {
                            output: Some(SemanticType::Scalar(ValueType::plain(narrow))),
                            kind: TypedExpressionKind::Literal(stored),
                            span: SPAN,
                        }),
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
        ],
        local_slot_count: 1,
        bindings: Vec::new(),
    }
}

/// Executes every statement of `program` and returns the local slots it leaves.
///
/// A rejected store can only be checked by inspecting the array value it did not
/// replace, which the public `execute` entry point does not expose.
fn execute_collecting_locals(
    program: &TypedProgram,
    registry: &Registry,
) -> (Result<(), RuntimeError>, Vec<Option<Value>>) {
    let mut runtime = Runtime {
        registry,
        configuration: registry.default_runtime_configuration(),
        local_values: vec![None; program.local_slot_count],
        loop_control: None,
    };
    let mut result = Ok(());
    for statement in &program.statements {
        if let Err(error) = runtime.execute_statement(statement) {
            result = Err(error);
            break;
        }
    }
    (result, runtime.local_values)
}

/// Returns the elements stored in one array local slot.
fn stored_elements(slot: &Option<Value>) -> &[Value] {
    slot.as_ref()
        .expect("the array binding is initialized")
        .downcast_ref::<ArrayValue>()
        .expect("the local slot holds an array")
        .elements()
}

/// Verifies a value that widened temporarily is normalized before storage.
#[test]
fn normalizes_a_temporarily_widened_element_before_storing_it() {
    let (registry, narrow, wide, _) = narrow_integer_registry();
    let program =
        store_boundary_program(narrow, ValueType::plain(narrow), Value::new(wide, 100_i16));

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    let elements = stored_elements(&locals[0]);
    assert_eq!(elements[0].type_id(), narrow);
    assert_eq!(elements[0].downcast_ref::<i8>(), Some(&100));
}

/// Verifies a value the declared representation cannot hold is rejected.
#[test]
fn rejects_a_widened_element_the_declared_representation_cannot_hold() {
    let (registry, narrow, wide, _) = narrow_integer_registry();
    let program =
        store_boundary_program(narrow, ValueType::plain(narrow), Value::new(wide, 128_i16));

    let (result, locals) = execute_collecting_locals(&program, &registry);

    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("array element `128` cannot be represented as `narrow`")
    );
    let elements = stored_elements(&locals[0]);
    assert_eq!(elements[0].type_id(), narrow);
    assert_eq!(elements[0].downcast_ref::<i8>(), Some(&127));
}

/// Verifies a normalized element keeps its unit and the declared subtype wins.
#[test]
fn keeps_the_element_unit_when_normalizing_a_widened_element() {
    let (registry, narrow, wide, millimeter) = narrow_integer_registry();
    let stored = Value::new(wide, 100_i16).with_subtype(Some(millimeter));
    let unconstrained = store_boundary_program(narrow, ValueType::plain(narrow), stored.clone());

    let (result, locals) = execute_collecting_locals(&unconstrained, &registry);

    result.unwrap();
    let elements = stored_elements(&locals[0]);
    assert_eq!(elements[0].subtype_id(), Some(millimeter));
    assert_eq!(elements[0].downcast_ref::<i8>(), Some(&100));

    let constrained =
        store_boundary_program(narrow, ValueType::qualified(narrow, millimeter), stored);
    let (result, locals) = execute_collecting_locals(&constrained, &registry);
    result.unwrap();
    let elements = stored_elements(&locals[0]);
    assert_eq!(elements[0].subtype_id(), Some(millimeter));
    assert_eq!(elements[0].downcast_ref::<i8>(), Some(&100));
}

/// Builds an indexed store whose value is a resolved subtype conversion.
fn conversion_store_program(
    registry: &Registry,
    narrow: crate::semantic::TypeId,
    millimeter: crate::semantic::SubtypeId,
    centimeter: crate::semantic::SubtypeId,
    conversion: ResolvedSubtypeConversion,
    value: i8,
) -> TypedProgram {
    let denominator_operator = registry
        .resolve_binary_operator(BinaryOperator::Division, narrow, narrow)
        .unwrap();
    let scale_plan = TypedScalePlan {
        numerator: None,
        denominator: Some(TypedScaleStep {
            operator: denominator_operator,
            factor: Value::new(narrow, 10_i8),
        }),
    };
    let target = ValueType::qualified(narrow, centimeter);
    let array = ArrayType {
        element: target,
        element_mode: ArrayElementMode::Exact,
    };
    let converted = TypedExpression {
        output: Some(SemanticType::Scalar(conversion.output)),
        kind: TypedExpressionKind::Convert {
            conversion,
            target_base: Some(narrow),
            scale_plan,
            expression: Box::new(TypedExpression {
                output: Some(SemanticType::Scalar(ValueType::qualified(
                    narrow, millimeter,
                ))),
                kind: TypedExpressionKind::Literal(
                    Value::new(narrow, value).with_subtype(Some(millimeter)),
                ),
                span: SPAN,
            }),
        },
        span: SPAN,
    };
    TypedProgram {
        statements: vec![
            TypedStatement::VariableDeclaration {
                name: "values".into(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                mutable: true,
                semantic_type: SemanticType::Array(array),
                expression: TypedExpression {
                    output: Some(SemanticType::Array(array)),
                    kind: TypedExpressionKind::ArrayLiteral {
                        elements: vec![TypedExpression {
                            output: Some(SemanticType::Scalar(target)),
                            kind: TypedExpressionKind::Literal(
                                Value::new(narrow, 2_i8).with_subtype(Some(centimeter)),
                            ),
                            span: SPAN,
                        }],
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
            TypedStatement::IndexedAssignment {
                name: "values".into(),
                binding: BindingId(0),
                slot: LocalVariableSlot(0),
                index: TypedExpression {
                    output: Some(SemanticType::Scalar(ValueType::plain(narrow))),
                    kind: TypedExpressionKind::Literal(Value::new(narrow, 0_i8)),
                    span: SPAN,
                },
                constant_index: Some(0),
                index_extractor: extract_narrow_index,
                index_dispatch: None,
                expression: TypedExpression {
                    output: Some(SemanticType::Scalar(target)),
                    kind: TypedExpressionKind::ElementStore {
                        element: target,
                        expression: Box::new(converted),
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
        ],
        local_slot_count: 1,
        bindings: Vec::new(),
    }
}

/// Verifies an inexact integer subtype conversion is rejected before division
/// truncates it, leaving the existing array element untouched.
#[test]
fn rejects_an_inexact_integer_subtype_conversion_before_storage() {
    let (registry, narrow, millimeter, centimeter) = exact_unit_conversion_registry();
    let conversion = registry
        .resolve_subtype_conversion(ValueType::qualified(narrow, millimeter), centimeter)
        .unwrap();
    let program =
        conversion_store_program(&registry, narrow, millimeter, centimeter, conversion, 15);

    let (result, locals) = execute_collecting_locals(&program, &registry);

    let error = result.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot convert `narrow` to `narrow`"),
        "actual runtime error: {error}"
    );
    let elements = stored_elements(&locals[0]);
    assert_eq!(elements[0].downcast_ref::<i8>(), Some(&2));
    assert_eq!(elements[0].subtype_id(), Some(centimeter));
}

/// Verifies an exact integer subtype conversion is evaluated and normalized.
#[test]
fn normalizes_an_exact_integer_subtype_conversion_before_storage() {
    let (registry, narrow, millimeter, centimeter) = exact_unit_conversion_registry();
    let conversion = registry
        .resolve_subtype_conversion(ValueType::qualified(narrow, millimeter), centimeter)
        .unwrap();
    let program =
        conversion_store_program(&registry, narrow, millimeter, centimeter, conversion, 20);

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    let elements = stored_elements(&locals[0]);
    assert_eq!(elements[0].downcast_ref::<i8>(), Some(&2));
    assert_eq!(elements[0].subtype_id(), Some(centimeter));
}

/// Marks the payload a null value carries in the array method tests.
#[derive(Clone, Copy)]
struct TestNull;

/// Parses the null literal into the test null payload.
fn parse_test_null(raw_text: &str, type_id: crate::semantic::TypeId) -> Result<Value, CoreError> {
    match raw_text {
        "null" => Ok(Value::new(type_id, TestNull)),
        _ => Err(CoreError::InvalidLiteral {
            raw_text: raw_text.to_string(),
            type_name: "null".to_string(),
            message: "expected `null`".to_string(),
        }),
    }
}

/// Builds a registry with one array element type and the language null type.
fn array_method_registry() -> (Registry, crate::semantic::TypeId, crate::semantic::TypeId) {
    let mut registry = Registry::new();
    let integer = registry.allocate_type_id();
    let null = registry.allocate_type_id();
    registry
        .register_type(TypeDescriptor {
            id: integer,
            name: "integer",
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
            id: null,
            name: "null",
            is_integer: false,
            parse_numeric_literal: None,
            parse_string_literal: None,
            parse_regex_literal: None,
            parse_boolean_literal: None,
            parse_null_literal: Some(parse_test_null),
            format: format_value,
        })
        .unwrap();
    registry.set_default_null(null).unwrap();
    registry.set_default_integer(integer).unwrap();
    (registry, integer, null)
}

/// Converts an integer fixture into a zero-based array index.
fn extract_integer_index(value: &Value) -> Result<usize, CoreError> {
    let index = value
        .downcast_ref::<i64>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("integer".into()))?;
    usize::try_from(index).map_err(|_| CoreError::Runtime(format!("index {index} is negative")))
}

/// Builds a statement that declares one array binding from literal elements.
fn array_declaration(
    slot: usize,
    integer: crate::semantic::TypeId,
    elements: Vec<i64>,
) -> TypedStatement {
    let element_type = ValueType::plain(integer);
    TypedStatement::VariableDeclaration {
        name: format!("values{slot}"),
        binding: BindingId(slot),
        slot: LocalVariableSlot(slot),
        mutable: true,
        semantic_type: SemanticType::Array(ArrayType {
            element: element_type,
            element_mode: ArrayElementMode::Exact,
        }),
        expression: TypedExpression {
            output: Some(SemanticType::Array(ArrayType {
                element: element_type,
                element_mode: ArrayElementMode::Exact,
            })),
            kind: TypedExpressionKind::ArrayLiteral {
                elements: elements
                    .into_iter()
                    .map(|value| TypedExpression {
                        output: Some(SemanticType::Scalar(element_type)),
                        kind: TypedExpressionKind::Literal(Value::new(integer, value)),
                        span: SPAN,
                    })
                    .collect(),
            },
            span: SPAN,
        },
        span: SPAN,
    }
}

/// Builds one expression that applies an array method to a local array slot.
fn array_method_expression(
    method: ArrayMethod,
    slot: usize,
    element: ValueType,
    arguments: Vec<TypedExpression>,
    empty_result: Option<Value>,
) -> TypedExpression {
    let output = match method.removes_element() {
        true => Some(SemanticType::Scalar(element)),
        false => None,
    };
    TypedExpression {
        output,
        kind: TypedExpressionKind::ArrayMethod {
            method,
            binding: BindingId(slot),
            slot: LocalVariableSlot(slot),
            arguments,
            result_domain: None,
            empty_result,
        },
        span: SPAN,
    }
}

/// Verifies a removal from an empty array produces the language null value.
#[test]
fn removing_from_an_empty_array_produces_the_null_value() {
    let (registry, integer, null) = array_method_registry();
    let element_type = ValueType::plain(integer);
    let program = TypedProgram {
        statements: vec![
            array_declaration(0, integer, Vec::new()),
            TypedStatement::VariableDeclaration {
                name: "removed".to_string(),
                binding: BindingId(1),
                slot: LocalVariableSlot(1),
                mutable: false,
                semantic_type: SemanticType::Scalar(element_type),
                expression: array_method_expression(
                    ArrayMethod::Pop,
                    0,
                    element_type,
                    Vec::new(),
                    Some(parse_test_null("null", null).expect("the null literal parses")),
                ),
                span: SPAN,
            },
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert_eq!(
        locals[1]
            .as_ref()
            .expect("the removal stored a value")
            .type_id(),
        null
    );
    assert!(stored_elements(&locals[0]).is_empty());
}

/// Verifies empty pop and shift do not copy a shared array before returning null.
#[test]
fn empty_removals_keep_shared_array_payloads() {
    let (registry, integer, null) = array_method_registry();
    let element_type = ValueType::plain(integer);
    let array_type = ArrayType {
        element: element_type,
        element_mode: ArrayElementMode::Exact,
    };
    let alias_declaration =
        |source_slot: usize, alias_slot: usize| TypedStatement::VariableDeclaration {
            name: format!("alias{alias_slot}"),
            binding: BindingId(alias_slot),
            slot: LocalVariableSlot(alias_slot),
            mutable: true,
            semantic_type: SemanticType::Array(array_type),
            expression: TypedExpression {
                output: Some(SemanticType::Array(array_type)),
                kind: TypedExpressionKind::Variable {
                    name: format!("values{source_slot}"),
                    binding: BindingId(source_slot),
                    slot: LocalVariableSlot(source_slot),
                    nullable: false,
                    complete_type_domain: None,
                },
                span: SPAN,
            },
            span: SPAN,
        };
    let program = TypedProgram {
        statements: vec![
            array_declaration(0, integer, Vec::new()),
            alias_declaration(0, 1),
            array_declaration(2, integer, Vec::new()),
            alias_declaration(2, 3),
            TypedStatement::Expression(array_method_expression(
                ArrayMethod::Pop,
                0,
                element_type,
                Vec::new(),
                Some(parse_test_null("null", null).expect("the null literal parses")),
            )),
            TypedStatement::Expression(array_method_expression(
                ArrayMethod::Shift,
                2,
                element_type,
                Vec::new(),
                Some(parse_test_null("null", null).expect("the null literal parses")),
            )),
        ],
        local_slot_count: 4,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    for (source_slot, alias_slot) in [(0, 1), (2, 3)] {
        let source = ArrayValue::from_value(locals[source_slot].as_ref().unwrap()).unwrap();
        let alias = ArrayValue::from_value(locals[alias_slot].as_ref().unwrap()).unwrap();
        assert!(source.elements().is_empty());
        assert_eq!(source as *const ArrayValue, alias as *const ArrayValue);
    }
}

/// Verifies an insertion through one binding leaves a shared array untouched.
///
/// Two bindings that hold the same array share one payload until one of them is
/// mutated, so the mutation has to copy the payload first. The test proves the
/// copy is made and that the other binding keeps its own elements.
#[test]
fn insertion_through_a_shared_binding_copies_the_array() {
    let (registry, integer, _) = array_method_registry();
    let element_type = ValueType::plain(integer);
    let program = TypedProgram {
        statements: vec![
            array_declaration(0, integer, vec![1, 2]),
            TypedStatement::VariableDeclaration {
                name: "alias".to_string(),
                binding: BindingId(1),
                slot: LocalVariableSlot(1),
                mutable: true,
                semantic_type: SemanticType::Array(ArrayType {
                    element: element_type,
                    element_mode: ArrayElementMode::Exact,
                }),
                expression: TypedExpression {
                    output: Some(SemanticType::Array(ArrayType {
                        element: element_type,
                        element_mode: ArrayElementMode::Exact,
                    })),
                    kind: TypedExpressionKind::Variable {
                        name: "values0".to_string(),
                        binding: BindingId(0),
                        slot: LocalVariableSlot(0),
                        nullable: false,
                        complete_type_domain: None,
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
            TypedStatement::Expression(array_method_expression(
                ArrayMethod::Push,
                1,
                element_type,
                vec![TypedExpression {
                    output: Some(SemanticType::Scalar(element_type)),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 3_i64)),
                    span: SPAN,
                }],
                None,
            )),
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    let original = stored_elements(&locals[0]);
    assert_eq!(original.len(), 2);
    assert_eq!(original[0].downcast_ref::<i64>(), Some(&1));
    assert_eq!(original[1].downcast_ref::<i64>(), Some(&2));
    let alias = stored_elements(&locals[1]);
    assert_eq!(alias.len(), 3);
    assert_eq!(alias[2].downcast_ref::<i64>(), Some(&3));
}

/// Verifies indexed assignment copies a shared payload and preserves its array identity.
#[test]
fn indexed_assignment_copies_a_shared_array_without_changing_its_identity() {
    let (registry, integer, _) = array_method_registry();
    let element_type = ValueType::plain(integer);
    let array_type = ArrayType {
        element: element_type,
        element_mode: ArrayElementMode::Exact,
    };
    let program = TypedProgram {
        statements: vec![
            array_declaration(0, integer, vec![1, 2]),
            TypedStatement::VariableDeclaration {
                name: "alias".into(),
                binding: BindingId(1),
                slot: LocalVariableSlot(1),
                mutable: true,
                semantic_type: SemanticType::Array(array_type),
                expression: TypedExpression {
                    output: Some(SemanticType::Array(array_type)),
                    kind: TypedExpressionKind::Variable {
                        name: "values0".into(),
                        binding: BindingId(0),
                        slot: LocalVariableSlot(0),
                        nullable: false,
                        complete_type_domain: None,
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
            TypedStatement::IndexedAssignment {
                name: "alias".into(),
                binding: BindingId(1),
                slot: LocalVariableSlot(1),
                index: TypedExpression {
                    output: Some(SemanticType::Scalar(element_type)),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 0_i64)),
                    span: SPAN,
                },
                constant_index: Some(0),
                index_extractor: extract_integer_index,
                index_dispatch: None,
                expression: TypedExpression {
                    output: Some(SemanticType::Scalar(element_type)),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 9_i64)),
                    span: SPAN,
                },
                span: SPAN,
            },
        ],
        local_slot_count: 2,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert_eq!(locals[0].as_ref().unwrap().array_type(), Some(array_type));
    assert_eq!(locals[1].as_ref().unwrap().array_type(), Some(array_type));
    let original = stored_elements(&locals[0]);
    assert_eq!(original[0].downcast_ref::<i64>(), Some(&1));
    let alias = stored_elements(&locals[1]);
    assert_eq!(alias[0].downcast_ref::<i64>(), Some(&9));
}

/// Verifies dynamic indexed reads use the array payload produced by evaluation.
#[test]
fn indexed_read_returns_the_element_at_a_runtime_index() {
    let (registry, integer, _) = array_method_registry();
    let element_type = ValueType::plain(integer);
    let array_type = ArrayType {
        element: element_type,
        element_mode: ArrayElementMode::Exact,
    };
    let array_expression = TypedExpression {
        output: Some(SemanticType::Array(array_type)),
        kind: TypedExpressionKind::Variable {
            name: "values".into(),
            binding: BindingId(0),
            slot: LocalVariableSlot(0),
            nullable: false,
            complete_type_domain: None,
        },
        span: SPAN,
    };
    let index_expression = TypedExpression {
        output: Some(SemanticType::Scalar(element_type)),
        kind: TypedExpressionKind::Variable {
            name: "index".into(),
            binding: BindingId(1),
            slot: LocalVariableSlot(1),
            nullable: false,
            complete_type_domain: None,
        },
        span: SPAN,
    };
    let program = TypedProgram {
        statements: vec![
            array_declaration(0, integer, vec![10, 20]),
            TypedStatement::VariableDeclaration {
                name: "index".into(),
                binding: BindingId(1),
                slot: LocalVariableSlot(1),
                mutable: false,
                semantic_type: SemanticType::Scalar(element_type),
                expression: TypedExpression {
                    output: Some(SemanticType::Scalar(element_type)),
                    kind: TypedExpressionKind::Literal(Value::new(integer, 1_i64)),
                    span: SPAN,
                },
                span: SPAN,
            },
            TypedStatement::VariableDeclaration {
                name: "selected".into(),
                binding: BindingId(2),
                slot: LocalVariableSlot(2),
                mutable: false,
                semantic_type: SemanticType::Scalar(element_type),
                expression: TypedExpression {
                    output: Some(SemanticType::Scalar(element_type)),
                    kind: TypedExpressionKind::ElementAccess {
                        array: Box::new(array_expression),
                        index: Box::new(index_expression),
                        constant_index: None,
                        index_extractor: extract_integer_index,
                        type_domain: None,
                        index_dispatch: None,
                    },
                    span: SPAN,
                },
                span: SPAN,
            },
        ],
        local_slot_count: 3,
        bindings: Vec::new(),
    };

    let (result, locals) = execute_collecting_locals(&program, &registry);

    result.unwrap();
    assert_eq!(locals[2].as_ref().unwrap().downcast_ref::<i64>(), Some(&20));
}
