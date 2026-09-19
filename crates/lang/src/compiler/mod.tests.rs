use crate::primitives::{BoolExtension, NullExtension, RegexExtension, StringExtension};
use crate::primitives::{DecimalExtension, IntegerExtension};
use crate::semantic::{
    ComparisonOperator, CoreError, Extension, Registry, TypeDescriptor, Value, ValueType,
};
use crate::syntax::{
    Block, ComparisonOperator as SyntaxComparisonOperator, ConfigurationEntry, ConfigurationValue,
    Expression, Program, Span, Statement,
};

use crate::{CompileError, TypedExpression, TypedExpressionKind, TypedStatement, compile};

const SPAN: Span = Span { start: 0, end: 1 };

fn parse_integer(raw_text: &str, type_id: crate::semantic::TypeId) -> Result<Value, CoreError> {
    Ok(Value::new(
        type_id,
        raw_text
            .parse::<i64>()
            .map_err(|error| CoreError::InvalidLiteral {
                raw_text: raw_text.into(),
                type_name: "int".into(),
                message: error.to_string(),
            })?,
    ))
}

fn parse_boolean(raw_text: &str, type_id: crate::semantic::TypeId) -> Result<Value, CoreError> {
    match raw_text {
        "true" => Ok(Value::new(type_id, true)),
        "false" => Ok(Value::new(type_id, false)),
        _ => Err(CoreError::InvalidLiteral {
            raw_text: raw_text.into(),
            type_name: "bool".into(),
            message: "expected a boolean".into(),
        }),
    }
}

fn evaluate_boolean(value: &Value) -> Result<bool, CoreError> {
    value
        .downcast_ref::<bool>()
        .copied()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("bool".into()))
}

fn format_value(_: &Value) -> Result<String, CoreError> {
    Ok(String::new())
}
fn compare_integers(left: &Value, right: &Value) -> Result<bool, CoreError> {
    let left = left
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))?;
    let right = right
        .downcast_ref::<i64>()
        .ok_or_else(|| CoreError::InvalidValueRepresentation("int".into()))?;
    Ok(left == right)
}

fn conditional_registry() -> Registry {
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
            parse_boolean_literal: Some(parse_boolean),
            parse_null_literal: None,
            format: format_value,
        })
        .unwrap();
    registry.set_default_integer(integer).unwrap();
    registry
        .set_default_boolean(boolean, evaluate_boolean)
        .unwrap();
    registry
}

fn number(raw_text: &str) -> Expression {
    Expression::Number {
        raw_text: raw_text.into(),
        suffix: None,
        span: SPAN,
    }
}

fn variable(name: &str) -> Expression {
    Expression::Variable {
        name: name.into(),
        span: SPAN,
    }
}

fn variable_declaration(name: &str, expression: Expression) -> Statement {
    Statement::VariableDeclaration {
        name: name.into(),
        type_name: "int".into(),
        expression,
        span: SPAN,
    }
}

fn if_statement(condition: Expression, statements: Vec<Statement>) -> Statement {
    Statement::If {
        condition,
        body: Block {
            statements,
            span: SPAN,
        },
        else_body: None,
        span: SPAN,
    }
}

fn compile_error(program: &Program, registry: &Registry) -> CompileError {
    match compile(program, registry) {
        Ok(_) => panic!("expected compilation to fail"),
        Err(error) => error,
    }
}

/// Builds the decimal registry used to inspect percentage lowering plans.
fn percentage_registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    crate::measures::PercentageMeasureExtension
        .register(&mut registry)
        .unwrap();
    registry
}

/// Verifies a percentage literal is normalized once while compiling its expression.
#[test]
fn folds_constant_percentage_scale_into_the_typed_literal() {
    let registry = percentage_registry();
    let syntax =
        crate::parser::parse("const value: decimal = 50\nconst result: decimal = value + 20%\n")
            .unwrap();
    let typed = compile(&syntax, &registry).unwrap();
    let TypedStatement::VariableDeclaration { expression, .. } = &typed.statements[1] else {
        panic!("expected result declaration");
    };
    let TypedExpressionKind::Binary {
        execution_plan,
        right_operand,
        ..
    } = &expression.kind
    else {
        panic!("expected percentage binary expression");
    };

    assert!(execution_plan.right_operand_scale.is_identity());
    assert!(execution_plan.relative_adjustment_operator.is_some());
    let TypedExpressionKind::Literal(value) = &right_operand.kind else {
        panic!("expected folded percentage literal");
    };
    assert_eq!(value.subtype_id(), None);
}

/// Verifies a dynamic percentage carries pre-resolved scaling and adjustment dispatch.
#[test]
fn prepares_dynamic_percentage_dispatch_during_compilation() {
    let registry = percentage_registry();
    let syntax =
        crate::parser::parse(
            "const value: decimal = 50\nconst rate: decimal = 20%\nconst result: decimal = value + rate\n",
        )
        .unwrap();
    let typed = compile(&syntax, &registry).unwrap();
    let TypedStatement::VariableDeclaration { expression, .. } = &typed.statements[2] else {
        panic!("expected result declaration");
    };
    let TypedExpressionKind::Binary { execution_plan, .. } = &expression.kind else {
        panic!("expected percentage binary expression");
    };

    assert!(execution_plan.right_operand_scale.numerator.is_none());
    assert!(execution_plan.right_operand_scale.denominator.is_some());
    assert!(execution_plan.relative_adjustment_operator.is_some());
}

/// Verifies that mixed string additions lower numeric operands to string calls.
#[test]
fn lowers_implicit_numeric_string_concatenation_in_both_orders() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    StringExtension.register(&mut registry).unwrap();
    let program = crate::parser::parse(
        "const left: string = 'value: ' + 3\nconst right: string = 3 + ' items'\n",
    )
    .unwrap();

    let typed = compile(&program, &registry).unwrap();

    let TypedStatement::VariableDeclaration {
        expression: left_expression,
        ..
    } = &typed.statements[0]
    else {
        panic!("expected the left string declaration");
    };
    let TypedExpressionKind::Binary {
        right_operand: converted_right_operand,
        ..
    } = &left_expression.kind
    else {
        panic!("expected left string concatenation");
    };
    assert!(matches!(
        converted_right_operand.kind,
        TypedExpressionKind::Call { .. }
    ));

    let TypedStatement::VariableDeclaration {
        expression: right_expression,
        ..
    } = &typed.statements[1]
    else {
        panic!("expected the right string declaration");
    };
    let TypedExpressionKind::Binary {
        left_operand: converted_left_operand,
        ..
    } = &right_expression.kind
    else {
        panic!("expected right string concatenation");
    };
    assert!(matches!(
        converted_left_operand.kind,
        TypedExpressionKind::Call { .. }
    ));
}

#[test]
fn comparisons_compile_to_a_plain_boolean_without_propagating_expected_type() {
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
            parse_boolean_literal: Some(parse_boolean),
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
            ComparisonOperator::Equal,
            integer,
            integer,
            compare_integers,
        )
        .unwrap();
    let program = Program {
        statements: vec![Statement::VariableDeclaration {
            name: "same".into(),
            type_name: "bool".into(),
            expression: Expression::Comparison {
                operator: SyntaxComparisonOperator::Equal,
                left_operand: Box::new(Expression::Number {
                    raw_text: "1".into(),
                    suffix: None,
                    span: SPAN,
                }),
                right_operand: Box::new(Expression::Number {
                    raw_text: "1".into(),
                    suffix: None,
                    span: SPAN,
                }),
                span: SPAN,
            },
            span: SPAN,
        }],
    };

    let typed = compile(&program, &registry).unwrap();
    let TypedStatement::VariableDeclaration {
        semantic_type,
        expression,
        ..
    } = &typed.statements[0]
    else {
        panic!("expected a variable declaration");
    };
    assert_eq!(
        *semantic_type,
        crate::semantic::SemanticType::Scalar(ValueType::plain(boolean))
    );
    assert!(matches!(
        expression.kind,
        TypedExpressionKind::Comparison { .. }
    ));
}

#[test]
fn if_blocks_see_outer_variables_and_release_their_local_scope() {
    let registry = conditional_registry();
    let program = Program {
        statements: vec![
            variable_declaration("outer", number("1")),
            if_statement(
                Expression::Boolean {
                    raw_text: "true".into(),
                    span: SPAN,
                },
                vec![variable_declaration("local", variable("outer"))],
            ),
            if_statement(
                Expression::Boolean {
                    raw_text: "false".into(),
                    span: SPAN,
                },
                vec![variable_declaration("local", number("2"))],
            ),
        ],
    };

    let typed = compile(&program, &registry).unwrap();

    let TypedStatement::VariableDeclaration {
        slot: outer_slot, ..
    } = &typed.statements[0]
    else {
        panic!("expected an outer variable declaration");
    };
    let TypedStatement::If {
        body: first_body, ..
    } = &typed.statements[1]
    else {
        panic!("expected the first conditional");
    };
    let TypedStatement::VariableDeclaration {
        slot: local_slot,
        expression,
        ..
    } = &first_body.statements[0]
    else {
        panic!("expected a local variable declaration");
    };
    let TypedExpressionKind::Variable {
        slot: referenced_outer_slot,
        ..
    } = expression.kind
    else {
        panic!("expected a local-slot variable reference");
    };
    let TypedStatement::If { .. } = &typed.statements[2] else {
        panic!("expected the second conditional");
    };

    assert_eq!(*outer_slot, referenced_outer_slot);
    assert_ne!(*outer_slot, *local_slot);
    assert_eq!(first_body.owned_slots.as_ref(), [*local_slot].as_slice());
    assert_eq!(typed.local_slot_count, 3);
}

#[test]
fn if_block_variables_do_not_escape_and_can_shadow_active_names() {
    let registry = conditional_registry();
    let escaping = Program {
        statements: vec![
            if_statement(
                Expression::Boolean {
                    raw_text: "true".into(),
                    span: SPAN,
                },
                vec![variable_declaration("local", number("1"))],
            ),
            Statement::Expression(variable("local")),
        ],
    };
    assert!(
        compile_error(&escaping, &registry)
            .message
            .contains("unknown binding `local`")
    );

    let shadowing = Program {
        statements: vec![
            variable_declaration("value", number("1")),
            if_statement(
                Expression::Boolean {
                    raw_text: "true".into(),
                    span: SPAN,
                },
                vec![variable_declaration("value", number("2"))],
            ),
        ],
    };
    let typed = compile(&shadowing, &registry).unwrap();
    let TypedStatement::VariableDeclaration {
        binding: outer_binding,
        ..
    } = &typed.statements[0]
    else {
        panic!("expected outer binding declaration");
    };
    let TypedStatement::If { body, .. } = &typed.statements[1] else {
        panic!("expected conditional");
    };
    let TypedStatement::VariableDeclaration {
        binding: inner_binding,
        slot: inner_slot,
        ..
    } = &body.statements[0]
    else {
        panic!("expected inner binding declaration");
    };
    assert_ne!(outer_binding, inner_binding);
    assert_eq!(body.owned_slots.as_ref(), [*inner_slot].as_slice());
}

/// Verifies nested lexical blocks retain only their directly declared slots.
#[test]
fn nested_blocks_record_direct_slots_in_declaration_order() {
    let registry = conditional_registry();
    let program = Program {
        statements: vec![if_statement(
            Expression::Boolean {
                raw_text: "true".into(),
                span: SPAN,
            },
            vec![
                variable_declaration("outer", number("1")),
                if_statement(
                    Expression::Boolean {
                        raw_text: "true".into(),
                        span: SPAN,
                    },
                    vec![variable_declaration("inner", number("2"))],
                ),
            ],
        )],
    };

    let typed = compile(&program, &registry).unwrap();
    let TypedStatement::If { body, .. } = &typed.statements[0] else {
        panic!("expected outer conditional");
    };
    let TypedStatement::If {
        body: nested_body, ..
    } = &body.statements[1]
    else {
        panic!("expected nested conditional");
    };
    assert_eq!(
        body.owned_slots.as_ref(),
        [crate::ir::LocalVariableSlot(0)].as_slice()
    );
    assert_eq!(
        nested_body.owned_slots.as_ref(),
        [crate::ir::LocalVariableSlot(1)].as_slice()
    );
}

/// Verifies mutable assignments retain their declaration identity and type.
#[test]
fn compiles_mutable_assignment_and_rejects_immutable_or_mismatched_updates() {
    let registry = conditional_registry();
    let mutable_program = Program {
        statements: vec![
            Statement::BindingDeclaration {
                kind: crate::syntax::BindingKind::Let,
                name: "value".into(),
                type_expression: None,
                expression: number("1"),
                span: SPAN,
            },
            Statement::Assignment {
                name: "value".into(),
                expression: number("2"),
                span: SPAN,
            },
        ],
    };
    let typed = compile(&mutable_program, &registry).unwrap();
    let TypedStatement::VariableDeclaration { binding, .. } = &typed.statements[0] else {
        panic!("expected declaration");
    };
    let TypedStatement::Assignment {
        binding: assignment_binding,
        ..
    } = &typed.statements[1]
    else {
        panic!("expected assignment");
    };
    assert_eq!(binding, assignment_binding);
    assert_eq!(typed.bindings.len(), 1);

    let immutable_program = Program {
        statements: vec![
            variable_declaration("fixed", number("1")),
            Statement::Assignment {
                name: "fixed".into(),
                expression: number("2"),
                span: SPAN,
            },
        ],
    };
    assert!(
        compile_error(&immutable_program, &registry)
            .message
            .contains("cannot assign to immutable binding `fixed`")
    );

    let duplicate_program = Program {
        statements: vec![
            Statement::BindingDeclaration {
                kind: crate::syntax::BindingKind::Const,
                name: "duplicate".into(),
                type_expression: None,
                expression: number("1"),
                span: SPAN,
            },
            Statement::BindingDeclaration {
                kind: crate::syntax::BindingKind::Let,
                name: "duplicate".into(),
                type_expression: None,
                expression: number("2"),
                span: SPAN,
            },
        ],
    };
    assert!(
        compile_error(&duplicate_program, &registry)
            .message
            .contains("binding `duplicate` is already declared in this scope")
    );
}

/// Verifies a scalar binding reports the dedicated array-assignment diagnostic.
#[test]
fn rejects_array_assignment_to_a_scalar_binding_with_specific_diagnostic() {
    let registry = conditional_registry();
    let program = crate::parser::parse("let value: int = 1\nvalue = [2]\n").unwrap();

    let error = compile_error(&program, &registry);

    assert!(
        error
            .message
            .contains("cannot assign an array to binding `value`")
    );
}

#[test]
fn if_conditions_must_produce_the_plain_default_boolean_type() {
    let registry = conditional_registry();
    let program = Program {
        statements: vec![if_statement(number("1"), Vec::new())],
    };

    let error = compile_error(&program, &registry);

    assert!(
        error
            .message
            .contains("if condition must produce `bool`, found `int`")
    );
}

/// Verifies ordinary logical operators require booleans and lower to typed logical IR.
#[test]
fn compiles_boolean_logical_operators_and_rejects_non_boolean_operands() {
    let registry = conditional_registry();
    let logical = Expression::Logical {
        operator: crate::syntax::LogicalOperator::And,
        left_operand: Box::new(Expression::Boolean {
            raw_text: "true".into(),
            span: SPAN,
        }),
        right_operand: Box::new(Expression::Unary {
            operator: crate::syntax::UnaryOperator::LogicalNot,
            operand: Box::new(Expression::Boolean {
                raw_text: "false".into(),
                span: SPAN,
            }),
            span: SPAN,
        }),
        span: SPAN,
    };
    let typed = compile(
        &Program {
            statements: vec![Statement::Expression(logical)],
        },
        &registry,
    )
    .unwrap();
    assert!(matches!(
        typed.statements[0],
        TypedStatement::Expression(TypedExpression {
            kind: TypedExpressionKind::Logical { .. },
            ..
        })
    ));

    let error = compile_error(
        &Program {
            statements: vec![Statement::Expression(Expression::Logical {
                operator: crate::syntax::LogicalOperator::Or,
                left_operand: Box::new(number("1")),
                right_operand: Box::new(Expression::Boolean {
                    raw_text: "true".into(),
                    span: SPAN,
                }),
                span: SPAN,
            })],
        },
        &registry,
    );
    assert!(error.message.contains("expected `bool`, found `int`"));
}

/// Verifies that manually constructed syntax cannot bypass the root-only configuration rule.
#[test]
fn rejects_configuration_directives_inside_compiled_blocks() {
    let registry = conditional_registry();
    let program = Program {
        statements: vec![if_statement(
            Expression::Boolean {
                raw_text: "true".into(),
                span: SPAN,
            },
            vec![Statement::Configuration {
                entries: vec![ConfigurationEntry {
                    name: "unknown".into(),
                    value: ConfigurationValue::Symbol {
                        name: "None".into(),
                        span: SPAN,
                    },
                    span: SPAN,
                }],
                span: SPAN,
            }],
        )],
    };

    let error = compile_error(&program, &registry);

    assert!(error.message.contains("only allowed at the root level"));
}

/// Verifies configuration flattening, precision capping, and `format: None` disabling.
#[test]
fn compiles_decimal_configuration_overrides() {
    let mut registry = Registry::new();
    DecimalExtension.register(&mut registry).unwrap();
    let program = crate::parser::parse(
        "@config {\n\
         decimal: {\n\
         precision: 29\n\
         scale: 29\n\
         format: None\n\
         }\n\
         }\n",
    )
    .unwrap();

    let typed = compile(&program, &registry).unwrap();
    let TypedStatement::Configuration {
        configuration_override,
        ..
    } = &typed.statements[0]
    else {
        panic!("expected a configuration directive");
    };
    let entries = configuration_override.entries().collect::<Vec<_>>();

    assert!(entries.contains(&(
        "decimal.precision",
        &crate::semantic::ConfigurationValue::Integer(28)
    )));
    assert!(entries.contains(&(
        "decimal.scale",
        &crate::semantic::ConfigurationValue::Integer(28)
    )));
    assert!(entries.contains(&(
        "decimal.format.scale",
        &crate::semantic::ConfigurationValue::None
    )));
    assert_eq!(entries.len(), 3);
}

/// Verifies compile-time rejection of invalid decimal configuration values and paths.
#[test]
fn rejects_invalid_decimal_configuration_overrides() {
    let mut registry = Registry::new();
    DecimalExtension.register(&mut registry).unwrap();

    for source in [
        "@config { decimal: { precision: None } }\n",
        "@config { decimal: { rounding: Unknown } }\n",
        "@config { decimal: { format: { scale: Max } } }\n",
        "@config { decimal: { unknown: 1 } }\n",
        "@config { decimal: None }\n",
    ] {
        let program = crate::parser::parse(source).unwrap();
        assert!(
            compile(&program, &registry).is_err(),
            "source unexpectedly compiled: {source}"
        );
    }
}

/// Builds the primitive registry needed by namespace import compiler tests.
fn string_namespace_registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    RegexExtension.register(&mut registry).unwrap();
    StringExtension.register(&mut registry).unwrap();
    registry
}

/// Compiles source using the String namespace test registry.
fn compile_string_namespace_source(source: &str) -> Result<crate::ir::TypedProgram, CompileError> {
    let registry = string_namespace_registry();
    let program = crate::parser::parse(source).unwrap();
    compile(&program, &registry)
}

/// Returns the function identity used by the first compiled call or pipe expression.
fn first_function_identity(program: &crate::ir::TypedProgram) -> crate::semantic::FunctionId {
    let expression = program
        .statements
        .iter()
        .find_map(|statement| match statement {
            TypedStatement::Expression(expression) => Some(expression),
            _ => None,
        })
        .expect("test program must contain an expression");
    match &expression.kind {
        TypedExpressionKind::Call { function, .. } | TypedExpressionKind::Pipe { function, .. } => {
            *function
        }
        _ => panic!("expected a function invocation"),
    }
}

/// Verifies every import form and chaining converge on one native FunctionId.
#[test]
fn resolves_all_string_invocation_forms_to_one_function_identity() {
    let sources = [
        "use String\nString.replace('a', 'a', 'b')",
        "use String as Str\nStr.replace('a', 'a', 'b')",
        "use { replace } from String\nreplace('a', 'a', 'b')",
        "use { replace as replace_text } from String\nreplace_text('a', 'a', 'b')",
        "use { replace as replace_text, lowercase } from String\nreplace_text(lowercase('A'), 'a', 'b')",
        "use * from String\nreplace(lowercase('A'), 'a', 'b')",
        "use * as Str from String\nStr.replace('a', 'a', 'b')",
        "'a' -> replace('a', 'b')",
    ];

    let registry = string_namespace_registry();
    let identities: Vec<_> = sources
        .iter()
        .map(|source| {
            let program = crate::parser::parse(source).unwrap();
            let typed = compile(&program, &registry)
                .unwrap_or_else(|error| panic!("failed to compile `{source}`: {error}"));
            first_function_identity(&typed)
        })
        .collect();
    assert!(identities.iter().all(|identity| *identity == identities[0]));
}

/// Verifies chained String lookup remains available without any import declaration.
#[test]
fn resolves_chained_string_functions_without_imports() {
    compile_string_namespace_source("'A' -> lowercase -> replace('a', 'b')").unwrap();
}

/// Verifies an unqualified namespaced function is unavailable until imported.
#[test]
fn rejects_unqualified_string_calls_without_imports() {
    let error = compile_string_namespace_source("replace('a', 'a', 'b')")
        .err()
        .expect("unimported call must fail");
    assert!(error.message.contains("not in scope"));
    assert_eq!(error.span, Span { start: 0, end: 7 });

    let qualified_error = compile_string_namespace_source("String.replace('a', 'a', 'b')")
        .err()
        .expect("unimported namespace must fail");
    assert!(
        qualified_error
            .message
            .contains("namespace `String` is not imported")
    );
}

/// Verifies unknown namespace and namespace-member errors retain focused spans.
#[test]
fn rejects_unknown_namespaces_and_members() {
    let unknown_namespace = compile_string_namespace_source("use Missing")
        .err()
        .expect("unknown namespace must fail");
    assert!(
        unknown_namespace
            .message
            .contains("unknown namespace `Missing`")
    );
    assert_eq!(unknown_namespace.span, Span { start: 4, end: 11 });

    let unknown_member = compile_string_namespace_source("use { missing } from String")
        .err()
        .expect("unknown member must fail");
    assert!(unknown_member.message.contains("has no member `missing`"));
    assert_eq!(unknown_member.span, Span { start: 6, end: 13 });
}

/// Verifies selective imports and aliases never silently replace earlier bindings.
#[test]
fn rejects_duplicate_imported_symbols_and_aliases() {
    for source in [
        "use { replace } from String\nuse { replace } from String",
        "use { replace as transform, lowercase as transform } from String",
        "use String as Text\nuse * as Text from String",
        "use { replace as Text } from String\nuse String as Text",
    ] {
        let error = compile_string_namespace_source(source)
            .err()
            .expect("duplicate import must fail");
        assert!(
            error.message.contains("already defined in this scope"),
            "unexpected error for `{source}`: {error}"
        );
    }
}

/// Verifies overlapping wildcard imports remain ambiguous until explicitly resolved.
#[test]
fn diagnoses_ambiguous_wildcard_imports_with_all_sources() {
    let mut registry = string_namespace_registry();
    registry.register_namespace("Regex", None).unwrap();
    registry
        .export_namespace_function("Regex", "replace", "String.replace")
        .unwrap();
    let program = crate::parser::parse(
        "use * from String\n\
         use * from Regex\n\
         replace('a', 'a', 'b')",
    )
    .unwrap();

    let error = compile(&program, &registry)
        .err()
        .expect("ambiguous wildcard call must fail");
    assert!(error.message.contains("ambiguous"));
    assert!(error.message.contains("Regex.replace"));
    assert!(error.message.contains("String.replace"));

    let duplicate_program = crate::parser::parse(
        "use { replace } from String\n\
         use { replace } from Regex",
    )
    .unwrap();
    let duplicate_error = compile(&duplicate_program, &registry)
        .err()
        .expect("selective collision must fail immediately");
    assert!(
        duplicate_error
            .message
            .contains("already defined in this scope")
    );
}

/// Verifies imports declared in a nested block do not escape that lexical scope.
#[test]
fn keeps_imports_lexically_scoped_to_blocks() {
    let error = compile_string_namespace_source(
        "if (true) {\n\
         use { lowercase } from String\n\
         lowercase('A')\n\
         }\n\
         lowercase('A')",
    )
    .err()
    .expect("block import must not escape");
    assert!(
        error.message.contains("not in scope"),
        "unexpected block-scope error: {error}"
    );
}

/// Verifies a compiled range stores the direct dispatch plan for its first type.
#[test]
fn compiles_integer_ranges_with_a_cached_execution_plan() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    let program = crate::parser::parse("for (index in 0..3) {}").unwrap();

    let typed = compile(&program, &registry).unwrap();
    let TypedStatement::For {
        variable_type,
        range_plan,
        ..
    } = &typed.statements[0]
    else {
        panic!("expected a compiled range loop");
    };

    assert_eq!(range_plan.current_type, *variable_type);
    assert_eq!(range_plan.increment_unit.value_type(), *variable_type);
}

/// Verifies decimal bounds are rejected from the explicit integral capability.
#[test]
fn rejects_non_integral_range_bounds_from_type_capability() {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    let program = crate::parser::parse("for (index in 0.5..3) {}\n").unwrap();

    let error = compile(&program, &registry)
        .err()
        .expect("decimal range bound must fail");

    assert!(error.message.contains("for range bounds must be integers"));
}

/// Builds the primitive registry required by nullable compiler tests.
fn nullable_registry() -> Registry {
    let mut registry = Registry::new();
    IntegerExtension.register(&mut registry).unwrap();
    DecimalExtension.register(&mut registry).unwrap();
    BoolExtension.register(&mut registry).unwrap();
    NullExtension.register(&mut registry).unwrap();
    StringExtension.register(&mut registry).unwrap();
    registry
}

/// Verifies nullable declarations accept null and ordinary values of their base type.
#[test]
fn compiles_constrained_nullable_bindings() {
    let registry = nullable_registry();
    let program = crate::parser::parse(
        "let missing: int? = null\nlet present: int? = 10\nconst enabled: bool? = true\n",
    )
    .unwrap();
    let typed = compile(&program, &registry).unwrap();
    assert_eq!(typed.bindings.len(), 3);
    assert!(typed.bindings.iter().all(|binding| binding.nullable));
}

/// Verifies non-nullable declarations reject the null literal.
#[test]
fn rejects_null_for_non_nullable_bindings() {
    let registry = nullable_registry();
    let program = crate::parser::parse("let value: int = null\n").unwrap();
    let error = compile(&program, &registry)
        .err()
        .expect("null must be rejected");
    assert!(
        error
            .message
            .contains("cannot assign null to non-nullable type `int`")
    );
}

/// Verifies nullable annotations do not expand into arbitrary registered unions.
#[test]
fn rejects_unsupported_nullable_base_types() {
    let mut registry = nullable_registry();
    RegexExtension.register(&mut registry).unwrap();
    let program = crate::parser::parse("let pattern: regex? = `/value/`\n").unwrap();
    let error = compile(&program, &registry)
        .err()
        .expect("regex nullability must remain unsupported");
    assert!(
        error
            .message
            .contains("nullable types are currently limited")
    );
}

/// Verifies direct inequality with null narrows only the true branch.
#[test]
fn narrows_nullable_bindings_inside_non_null_branches() {
    let registry = nullable_registry();
    let accepted = crate::parser::parse(
        "let value: int? = 10\nif (value != null) {\nlet sum = value + 1\n}\n",
    )
    .unwrap();
    compile(&accepted, &registry).unwrap();

    let rejected = crate::parser::parse(
        "let value: int? = 10\nif (value != null) { let sum = value + 1 }\nlet invalid = value + 1\n",
    )
    .unwrap();
    let error = compile(&rejected, &registry)
        .err()
        .expect("narrowing must not escape");
    assert!(error.message.contains("nullable value must be narrowed"));
}

/// Verifies a repeated nested guard does not discard the enclosing non-null proof.
#[test]
fn preserves_enclosing_narrowing_after_nested_guard() {
    let registry = nullable_registry();
    let program = crate::parser::parse(
        "let value: int? = 10\n\
         if (value != null) {\n\
         if (value != null) {\n\
         let inner = value + 1\n\
         }\n\
         let outer = value + 1\n\
         }\n",
    )
    .unwrap();

    compile(&program, &registry).unwrap();
}

/// Verifies assignment invalidates narrowing without panicking when the branch ends.
#[test]
fn invalidates_narrowing_after_nullable_assignment() {
    let registry = nullable_registry();
    for assigned_value in ["null", "2"] {
        let source = format!(
            "let value: int? = 1\n\
             if (value != null) {{\n\
             value = {assigned_value}\n\
             let invalid = value + 1\n\
             }}\n"
        );
        let program = crate::parser::parse(&source).unwrap();
        let error = compile(&program, &registry)
            .err()
            .expect("assignment must invalidate the active proof");
        assert!(error.message.contains("nullable value must be narrowed"));
    }
}

/// Verifies a range iterator may shadow a binding in its parent lexical scope.
#[test]
fn allows_for_iterator_to_shadow_outer_binding() {
    let registry = nullable_registry();
    let program = crate::parser::parse(
        "let index: int = 10\n\
         for (index in 0..2) {\n\
         let current = index\n\
         }\n\
         let retained = index\n",
    )
    .unwrap();

    compile(&program, &registry).unwrap();
}
