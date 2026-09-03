use super::*;
use crate::lexer::lex;
use syntax::{ConfigurationValue, Expression, Program, Statement, UseClause};

fn parse(source: &str) -> Result<Program, ParseError> {
    Parser::new(lex(source)?).parse_program()
}

#[test]
fn parses_multiline_and_nested_if_statements() {
    let program = parse(
        "if (true) {\n\
         value: int = 1\n\
         if (value == 1) { print(value) }\n\
         }\n",
    )
    .unwrap();

    let Statement::If {
        condition, body, ..
    } = &program.statements[0]
    else {
        panic!("expected an if statement");
    };
    assert!(matches!(condition, Expression::Boolean { raw_text, .. } if raw_text == "true"));
    assert_eq!(body.statements.len(), 2);
    assert!(matches!(body.statements[1], Statement::If { .. }));
}

#[test]
fn accepts_a_newline_between_the_condition_and_block() {
    let program = parse("if (true)\n{\nprint(1)\n}\n").unwrap();

    assert!(matches!(program.statements[0], Statement::If { .. }));
}

#[test]
fn accepts_an_empty_block_and_preserves_if_and_block_spans() {
    let program = parse("if (true) {}").unwrap();

    let Statement::If { body, span, .. } = &program.statements[0] else {
        panic!("expected an if statement");
    };
    assert!(body.statements.is_empty());
    assert_eq!(*span, syntax::Span { start: 0, end: 12 });
    assert_eq!(body.span, syntax::Span { start: 10, end: 12 });
}

#[test]
fn rejects_unclosed_and_non_separated_blocks() {
    let unclosed = parse("if (true) {\nprint(1)\n").unwrap_err();
    assert!(unclosed.message.contains("expected `}`"));

    let non_separated = parse("if (true) { print(1) print(2) }\n").unwrap_err();
    assert!(
        non_separated
            .message
            .contains("expected end of line or `}`")
    );
}

#[test]
fn rejects_missing_condition_parentheses_and_opening_brace() {
    for (source, expected) in [
        ("if true) {}", "expected LeftParenthesis"),
        ("if (true {}", "expected RightParenthesis"),
        ("if (true)\nprint(1)", "expected LeftBrace"),
    ] {
        let error = parse(source).unwrap_err();
        assert!(
            error.message.contains(expected),
            "unexpected error for `{source}`: {error}"
        );
    }
}

/// Verifies that a root configuration directive preserves its nested object structure.
#[test]
fn parses_nested_root_configuration_directives() {
    let program = parse(
        "@config {\n\
         decimal: {\n\
         precision: 29\n\
         rounding: HalfEven\n\
         format: { scale: 4\nrounding: Truncate }\n\
         }\n\
         }\n",
    )
    .unwrap();

    let Statement::Configuration { entries, .. } = &program.statements[0] else {
        panic!("expected a configuration directive");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "decimal");
    let ConfigurationValue::Object {
        entries: decimal_entries,
        ..
    } = &entries[0].value
    else {
        panic!("expected the decimal configuration object");
    };
    assert!(matches!(
        decimal_entries[0].value,
        ConfigurationValue::Number { ref raw_text, .. } if raw_text == "29"
    ));
    assert!(matches!(
        decimal_entries[1].value,
        ConfigurationValue::Symbol { ref name, .. } if name == "HalfEven"
    ));
    assert!(matches!(
        decimal_entries[2].value,
        ConfigurationValue::Object { .. }
    ));
}

/// Verifies that configuration directives cannot be nested in control-flow blocks.
#[test]
fn rejects_configuration_directives_inside_blocks() {
    let error = parse("if (true) {\n@config { decimal: { precision: 4 } }\n}\n").unwrap_err();

    assert!(error.message.contains("only allowed at the root level"));
}

/// Verifies structural row types and native non-generic frame declarations.
#[test]
fn parses_type_and_frame_declarations() {
    let program = parse(
        "type Employee {\n\
         id: int\n\
         name: string\n\
         }\n\
         employees: frame Employee\n",
    )
    .unwrap();

    let Statement::TypeDeclaration { definition, .. } = &program.statements[0] else {
        panic!("expected a type declaration");
    };
    assert_eq!(definition.name, "Employee");
    assert_eq!(definition.fields.len(), 2);
    let Statement::FrameDeclaration {
        name,
        row_type_name,
        expression,
        ..
    } = &program.statements[1]
    else {
        panic!("expected a frame declaration");
    };
    assert_eq!(name, "employees");
    assert_eq!(row_type_name, "Employee");
    assert!(expression.is_none());
}

/// Verifies a frame declaration retains hand-written column-oriented literal data.
#[test]
fn parses_hand_written_frame_literals() {
    let program = parse(
        "employees: frame Employee = frame {\n\
         id: [1, 2, 3]\n\
         name: [\"Mario\", \"Anna\", \"Sara\"]\n\
         }\n",
    )
    .unwrap();

    let Statement::FrameDeclaration {
        expression: Some(Expression::FrameLiteral { columns, .. }),
        ..
    } = &program.statements[0]
    else {
        panic!("expected an initialized frame declaration");
    };
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].name, "id");
    assert_eq!(columns[0].values.len(), 3);
}

/// Verifies composite predicates and explicit relation bindings preserve every role.
#[test]
fn parses_composite_relation_definitions_and_bindings() {
    let program = parse(
        "relation CustomerOrders {\n\
         customer: Customer one\n\
         orders: Order many\n\
         on {\n\
         orders.customer_id == customer.id\n\
         orders.company_id == customer.company_id\n\
         }\n\
         }\n\
         relation sales: CustomerOrders {\n\
         customer = customers\n\
         orders = orders2026\n\
         }\n",
    )
    .unwrap();

    let Statement::RelationDefinition { definition, .. } = &program.statements[0] else {
        panic!("expected a relation definition");
    };
    assert_eq!(definition.roles.len(), 2);
    assert_eq!(definition.predicates.len(), 2);
    assert!(matches!(
        definition.predicates[0],
        Expression::Comparison { .. }
    ));
    let Statement::RelationBinding { binding, .. } = &program.statements[1] else {
        panic!("expected a relation binding");
    };
    assert_eq!(binding.definition_name, "CustomerOrders");
    assert_eq!(binding.roles.len(), 2);
}

/// Verifies relation definitions support more than two independently named participants.
#[test]
fn parses_n_ary_relation_definitions() {
    let program = parse(
        "relation OrderContext {\n\
         order: Order many\n\
         customer: Customer one\n\
         product: Product one\n\
         on {\n\
         order.customer_id == customer.id\n\
         order.product_id == product.id\n\
         }\n\
         }\n",
    )
    .unwrap();

    let Statement::RelationDefinition { definition, .. } = &program.statements[0] else {
        panic!("expected a relation definition");
    };
    assert_eq!(definition.roles.len(), 3);
    assert_eq!(definition.predicates.len(), 2);
}

/// Verifies invalid cardinality spelling is rejected by the syntax layer.
#[test]
fn rejects_invalid_relation_cardinality() {
    let error = parse(
        "relation CustomerOrders {\n\
         customer: Customer optional\n\
         on { customer.id == customer.id }\n\
         }\n",
    )
    .unwrap_err();

    assert!(error.message.contains("invalid cardinality"));
}

/// Verifies every supported `use` form maps to its explicit AST clause.
#[test]
fn parses_namespace_member_and_wildcard_imports() {
    let program = parse(
        "use String\n\
         use String as Str\n\
         use { replace, lowercase as lower } from String\n\
         use * from String\n\
         use * as Text from String\n",
    )
    .unwrap();

    let Statement::Use(first) = &program.statements[0] else {
        panic!("expected a namespace import");
    };
    assert_eq!(first.use_span, syntax::Span { start: 0, end: 3 });
    assert!(matches!(
        &first.clause,
        UseClause::Namespace { namespace, alias: None } if namespace.name == "String"
    ));
    assert!(matches!(
        &program.statements[1],
        Statement::Use(declaration)
            if matches!(&declaration.clause, UseClause::Namespace {
                namespace,
                alias: Some(alias),
            } if namespace.name == "String" && alias.name == "Str")
    ));
    let Statement::Use(members) = &program.statements[2] else {
        panic!("expected a member import");
    };
    let UseClause::Members { namespace, members } = &members.clause else {
        panic!("expected the members clause");
    };
    assert_eq!(namespace.name, "String");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].name.name, "replace");
    assert_eq!(members[1].alias.as_ref().unwrap().name, "lower");
    assert!(matches!(
        &program.statements[3],
        Statement::Use(declaration)
            if matches!(&declaration.clause, UseClause::Wildcard { alias: None, .. })
    ));
    assert!(matches!(
        &program.statements[4],
        Statement::Use(declaration)
            if matches!(&declaration.clause, UseClause::Wildcard {
                alias: Some(alias), ..
            } if alias.name == "Text")
    ));
}

/// Verifies a namespace-qualified call retains separate namespace and member spans.
#[test]
fn parses_namespace_qualified_calls() {
    let program = parse("String.replace('a', 'a', 'b')").unwrap();
    let Statement::Expression(Expression::Call {
        namespace: Some(namespace),
        function,
        ..
    }) = &program.statements[0]
    else {
        panic!("expected a qualified call");
    };
    assert_eq!(namespace.name, "String");
    assert_eq!(namespace.span, syntax::Span { start: 0, end: 6 });
    assert_eq!(function.name, "replace");
    assert_eq!(function.span, syntax::Span { start: 7, end: 14 });
}

/// Verifies malformed import clauses fail at the syntax layer.
#[test]
fn rejects_invalid_use_syntax() {
    for source in [
        "use",
        "use {} from String",
        "use { replace, } from String",
        "use * String",
        "use String as",
    ] {
        assert!(parse(source).is_err(), "`{source}` should be rejected");
    }
}
