use language_core::{Extension, Registry, ValueType};
use measures::MeasuresExtension;

use crate::{CompileError, TypedStatement, compile};
use ir::{ArrayType, TypedExpression, TypedExpressionKind, TypedProgram};

/// Builds a registry with every primitive and measure type for array tests.
///
/// The measure extension supplies the qualified subtypes and bidirectional
/// conversions the constrained-element tests rely on, so the tests exercise the
/// same registry contracts the CLI dialect installs.
fn array_registry() -> Registry {
    let mut registry = Registry::new();
    primitives::register_all(&mut registry).unwrap();
    MeasuresExtension.register(&mut registry).unwrap();
    registry
}

/// Compiles one source program or panics with the reported diagnostic.
fn compile_source(source: &str) -> TypedProgram {
    let program = parser::parse(source).expect("source must parse");
    compile(&program, &array_registry()).expect("source must compile")
}

/// Returns the compiler diagnostic produced by one rejected source program.
fn compile_error(source: &str) -> CompileError {
    let program = parser::parse(source).expect("source must parse");
    match compile(&program, &array_registry()) {
        Ok(_) => panic!("source must be rejected"),
        Err(error) => error,
    }
}

/// Returns the element contract of the first array binding in `program`.
fn first_array_type(program: &TypedProgram) -> ArrayType {
    for statement in &program.statements {
        if let TypedStatement::VariableDeclaration { expression, .. } = statement
            && let Some(array_type) = expression.array_type()
        {
            return array_type;
        }
    }
    panic!("program declares no array binding");
}

/// Returns the constant index and output type of the first element read.
fn first_element_access(program: &TypedProgram) -> (Option<usize>, ValueType) {
    for statement in &program.statements {
        if let TypedStatement::VariableDeclaration { expression, .. } = statement
            && let TypedExpressionKind::ElementAccess { constant_index, .. } = &expression.kind
        {
            return (
                *constant_index,
                expression.output.expect("element access produces a value"),
            );
        }
    }
    panic!("program reads no array element");
}

/// Verifies a typed array declaration keeps its constrained element subtype.
#[test]
fn keeps_declared_element_subtype() {
    let registry = array_registry();
    let millimeter = registry.subtype_by_suffix("mm").expect("mm is registered");
    let program = parser::parse("let sizes: int<mm>[] = [10mm, 20mm, 30mm]\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let array_type = first_array_type(&program);
    assert_eq!(array_type.element.subtype, Some(millimeter));
    assert!(array_type.adaptive_integer);
}

/// Verifies `int64[]` is fixed while `int[]` is adaptive.
#[test]
fn distinguishes_adaptive_int_from_fixed_width() {
    let registry = array_registry();
    let adaptive_program = parser::parse("let values: int[] = [1]\n").unwrap();
    let fixed_program = parser::parse("let values: int64[] = [1]\n").unwrap();
    let adaptive = first_array_type(&compile(&adaptive_program, &registry).unwrap());
    let fixed = first_array_type(&compile(&fixed_program, &registry).unwrap());
    assert!(adaptive.adaptive_integer);
    assert!(!fixed.adaptive_integer);
    assert_eq!(adaptive.element.base, fixed.element.base);
}

/// Verifies an inferred literal keeps an identical complete element type.
#[test]
fn infers_identical_complete_element_type() {
    let program = compile_source("let sizes = [10mm, 20mm, 30mm]\n");
    let array_type = first_array_type(&program);
    assert!(array_type.element.subtype.is_some());
    assert!(array_type.adaptive_integer);
}

/// Verifies differing subtypes infer the common base with no element subtype.
#[test]
fn infers_common_base_for_differing_subtypes() {
    let program = compile_source("let sizes = [10mm, 2cm, 3dm]\n");
    let array_type = first_array_type(&program);
    assert_eq!(array_type.element.subtype, None);
    assert!(array_type.adaptive_integer);
}

/// Verifies an untyped integer literal array infers the adaptive `int` element.
#[test]
fn infers_adaptive_integer_elements() {
    let program = compile_source("let numbers = [1, 2, 3]\n");
    let array_type = first_array_type(&program);
    assert_eq!(array_type.element.subtype, None);
    assert!(array_type.adaptive_integer);
}

/// Verifies an empty literal without an annotation cannot infer a type.
#[test]
fn rejects_empty_array_without_annotation() {
    let error = compile_error("let values = []\n");
    assert!(error.message.contains("empty array"));
}

/// Verifies a typed empty array compiles from its declared element contract.
#[test]
fn accepts_typed_empty_array() {
    let program = compile_source("let values: int[] = []\n");
    assert_eq!(first_array_type(&program).element.subtype, None);
}

/// Verifies elements that share no base type cannot form an array.
#[test]
fn rejects_elements_with_incompatible_bases() {
    let error = compile_error("let values = [10mm, \"hello\"]\n");
    assert!(error.message.contains("share a base type"));
}

/// Verifies compatible elements are converted to the constrained subtype.
#[test]
fn converts_compatible_elements_to_declared_subtype() {
    let program = compile_source("let sizes: int<mm>[] = [10mm, 2cm, 1dm]\n");
    let TypedStatement::VariableDeclaration { expression, .. } = &program.statements[0] else {
        panic!("expected an array declaration");
    };
    let TypedExpressionKind::ArrayLiteral { elements, .. } = &expression.kind else {
        panic!("expected an array literal");
    };
    assert_eq!(elements.len(), 3);
    assert!(matches!(
        elements[1].kind,
        TypedExpressionKind::Convert { .. }
    ));
}

/// Verifies elements without a registered conversion to the subtype are invalid.
#[test]
fn rejects_elements_with_incompatible_subtype() {
    let error = compile_error("let sizes: int<mm>[] = [10mm, 5kg]\n");
    assert!(
        error.message.contains("conversion") || error.message.contains("subtype"),
        "unexpected diagnostic: {}",
        error.message
    );
}

/// Verifies a literal index is resolved to a constant element offset.
#[test]
fn resolves_constant_element_index() {
    let program = compile_source("let values: int[] = [10, 20]\nlet second = values[1]\n");
    let (constant_index, output) = first_element_access(&program);
    assert_eq!(constant_index, Some(1));
    assert_eq!(output.subtype, None);
}

/// Verifies a qualified element type is preserved when the array constrains it.
#[test]
fn preserves_constrained_element_type_on_read() {
    let program = compile_source("let sizes: int<mm>[] = [10mm]\nlet first = sizes[0]\n");
    let (_, output) = first_element_access(&program);
    assert!(output.subtype.is_some());
}

/// Verifies a constant read keeps an unconstrained element's own subtype.
#[test]
fn preserves_unconstrained_element_subtype_for_constant_read() {
    let program = compile_source("let sizes: int[] = [10mm, 2cm]\nlet first = sizes[0]\n");
    let first = program
        .bindings
        .iter()
        .find(|binding| binding.name == "first")
        .expect("the element binding is recorded");
    assert!(
        first.value_type.subtype.is_some(),
        "a constant element read keeps the stored element subtype"
    );
}

/// Verifies a conditional write does not make an outer constant read assume its body ran.
#[test]
fn marks_a_conditionally_written_element_dynamic() {
    let program = compile_source(
        "let sizes: int[] = [10mm]\n\
         if (false) {\n\
         sizes[0] = 2cm\n\
         }\n\
         let total = sizes[0] + 1mm\n",
    );
    let total = binding_initializer(&program, "total");
    assert!(matches!(
        total.kind,
        TypedExpressionKind::DynamicBinary { .. }
    ));
}

/// Verifies a dynamic element write removes the per-element type record.
#[test]
fn drops_element_record_after_dynamic_write() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         let index: int = 1\n\
         sizes[index] = 3dm\n\
         let read = sizes[index]\n",
    );
    let read = program
        .bindings
        .iter()
        .find(|binding| binding.name == "read")
        .expect("the read binding is recorded");
    assert_eq!(
        read.value_type.subtype, None,
        "a dynamic write falls back to the declared element contract"
    );
}

/// Verifies a non-integer index is rejected.
#[test]
fn rejects_non_integer_index() {
    let error = compile_error("let values: int[] = [1]\nlet value = values[\"0\"]\n");
    assert!(error.message.contains("plain integer"));
}

/// Verifies an array cannot be used as a scalar operator operand.
#[test]
fn rejects_array_as_scalar_operand() {
    let error = compile_error("let values: int[] = [1]\nlet total = values + 1\n");
    assert!(error.message.contains("array"));
}

/// Verifies an array cannot initialize a scalar binding.
#[test]
fn rejects_array_in_scalar_binding() {
    let error = compile_error("let value: int = [1, 2]\n");
    assert!(error.message.contains("array"));
}

/// Verifies nested array literals are rejected instead of mis-inferred.
#[test]
fn rejects_nested_array_literals() {
    let error = compile_error("let values = [[1, 2], [3, 4]]\n");
    assert!(error.message.contains("nested arrays"));
}

/// Verifies indexed mutation is accepted on a mutable array binding.
#[test]
fn accepts_indexed_mutation_on_mutable_array() {
    compile_source("let values: int[] = [1, 2]\nvalues[0] = 5\n");
}

/// Verifies indexed mutation through a `const` array is rejected.
#[test]
fn rejects_indexed_mutation_through_const_array() {
    let error = compile_error("const values: int[] = [1, 2]\nvalues[0] = 5\n");
    assert!(error.message.contains("immutable"));
}

/// Verifies a fixed-width array rejects a value that exceeds its declared width.
#[test]
fn rejects_fixed_width_element_overflow() {
    let error =
        compile_error("let values: int64[] = [1]\nvalues[0] = 999999999999999999999999999999\n");
    assert!(error.message.contains("int64") || error.message.contains("literal"));
}

/// Verifies an adaptive `int` array accepts a value wider than `int64`.
#[test]
fn widens_adaptive_int_element_beyond_declared_width() {
    let program =
        compile_source("let values: int[] = [1]\nvalues[0] = 999999999999999999999999999999\n");
    let TypedStatement::IndexedAssignment { expression, .. } = &program.statements[1] else {
        panic!("expected an indexed assignment");
    };
    let output = expression.output.expect("the element produces a value");
    assert_ne!(output.base, first_array_type(&program).element.base);
}

/// Verifies a fixed-width store carries the declared element contract.
///
/// Reading an `int8` element and adding to it may promote the result while it is
/// evaluated, so the store must carry the destination the value has to satisfy.
#[test]
fn embeds_the_declared_contract_in_a_fixed_width_store() {
    let registry = array_registry();
    let int8 = registry.type_by_name("int8").expect("int8 is registered");
    let program = parser::parse("let values: int8[] = [127]\nvalues[0] = values[0] + 1\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = first_indexed_assignment_element(&program);
    let TypedExpressionKind::ElementStore {
        element: destination,
        expression,
    } = &element.kind
    else {
        panic!("a fixed-width store must carry its destination contract");
    };
    assert_eq!(*destination, ValueType::plain(int8));
    assert_eq!(expression.output, Some(ValueType::plain(int8)));
}

/// Verifies a provably representable literal store needs no runtime contract.
#[test]
fn stores_a_fixed_width_literal_without_a_contract_node() {
    let program = compile_source("let values: int8[] = [10]\nvalues[0] = 20\n");
    let element = first_indexed_assignment_element(&program);
    assert!(matches!(element.kind, TypedExpressionKind::Literal(_)));
}

/// Verifies a fixed-width element read is trusted at a fixed-width destination.
///
/// Element storage only ever holds values the declared representation accepted,
/// so a read of such an array needs no second check when it is stored again.
#[test]
fn stores_a_proven_fixed_width_read_without_a_contract_node() {
    let program = compile_source("let source: int8[] = [10]\nlet copy: int8[] = [source[0]]\n");
    let elements = array_literal_elements(binding_initializer(&program, "copy"));
    assert!(matches!(
        elements[0].kind,
        TypedExpressionKind::ElementAccess { .. }
    ));
}

/// Verifies an adaptive `int` store keeps its auto-widening semantics.
#[test]
fn keeps_adaptive_integer_stores_out_of_the_fixed_width_boundary() {
    let registry = array_registry();
    let program = parser::parse("let values: int[] = [1]\nvalues[0] = values[0] + 1\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = first_indexed_assignment_element(&program);
    assert!(matches!(element.kind, TypedExpressionKind::Binary { .. }));
}

/// Verifies a constrained store carries the declared unit as part of its contract.
#[test]
fn embeds_the_declared_subtype_in_a_constrained_store() {
    let registry = array_registry();
    let int8 = registry.type_by_name("int8").expect("int8 is registered");
    let millimeter = registry.subtype_by_suffix("mm").expect("mm is registered");
    let program =
        parser::parse("let sizes: int8<mm>[] = [10mm]\nsizes[0] = sizes[0] + 1mm\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = first_indexed_assignment_element(&program);
    let TypedExpressionKind::ElementStore { element, .. } = &element.kind else {
        panic!("a fixed-width subtype store must carry its destination contract");
    };
    assert_eq!(*element, ValueType::qualified(int8, millimeter));
}

/// Verifies an inferred fixed-width element is checked like a declared one.
#[test]
fn checks_an_inferred_fixed_width_array_element() {
    let program = compile_source("let x: int8 = 127\nlet y: int8 = 1\nlet values = [x + y]\n");
    let elements = array_literal_elements(binding_initializer(&program, "values"));
    assert!(matches!(
        elements[0].kind,
        TypedExpressionKind::ElementStore { .. }
    ));
}

/// Returns the element expression of the first indexed assignment in `program`.
fn first_indexed_assignment_element(program: &TypedProgram) -> &TypedExpression {
    for statement in &program.statements {
        if let TypedStatement::IndexedAssignment { expression, .. } = statement {
            return expression;
        }
    }
    panic!("program contains no indexed assignment");
}

/// Returns the compiled elements of an array literal expression.
fn array_literal_elements(expression: &TypedExpression) -> &[TypedExpression] {
    match &expression.kind {
        TypedExpressionKind::ArrayLiteral { elements, .. } => elements,
        _ => panic!("expression is not an array literal"),
    }
}

/// Verifies whole-array reassignment is rejected in favor of element assignment.
#[test]
fn rejects_whole_array_reassignment() {
    let error = compile_error("let values: int[] = [1]\nvalues = [2]\n");
    assert!(error.message.contains("whole-array reassignment"));
}

/// Returns the initializer of one named binding in a compiled program.
fn binding_initializer<'program>(
    program: &'program TypedProgram,
    name: &str,
) -> &'program TypedExpression {
    for statement in &program.statements {
        if let TypedStatement::VariableDeclaration {
            name: binding,
            expression,
            ..
        } = statement
            && binding == name
        {
            return expression;
        }
    }
    panic!("program declares no binding `{name}`");
}

/// Verifies a read after a dynamic write still dispatches on the stored subtype.
#[test]
fn marks_dynamic_element_read_after_dynamic_write() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         let index = 1\n\
         sizes[index] = 3dm\n\
         let read = sizes[index]\n",
    );
    let read = binding_initializer(&program, "read");
    assert!(matches!(
        read.kind,
        TypedExpressionKind::ElementAccess {
            dynamic_subtype: true,
            ..
        }
    ));
}

/// Verifies a dynamic read without any prior write is dynamic as well.
#[test]
fn marks_dynamic_element_read_without_static_record() {
    let program =
        compile_source("let sizes: int[] = [10mm, 2cm]\nlet index = 0\nlet read = sizes[index]\n");
    let read = binding_initializer(&program, "read");
    assert!(matches!(
        read.kind,
        TypedExpressionKind::ElementAccess {
            dynamic_subtype: true,
            ..
        }
    ));
}

/// Verifies arithmetic on a dynamic element pre-resolves one plan per subtype.
#[test]
fn dispatches_dynamic_element_arithmetic() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         let index = 1\n\
         sizes[index] = 3dm\n\
         let total = sizes[index] + 2dm\n",
    );
    let total = binding_initializer(&program, "total");
    let TypedExpressionKind::DynamicBinary { dispatch, .. } = &total.kind else {
        panic!("expected a dynamic binary operation");
    };
    assert!(
        dispatch.left_width > 1,
        "the dynamic operand contributes one slot per candidate subtype"
    );
    assert_eq!(dispatch.right_width, 1);
    assert!(
        dispatch.plans.iter().any(|plan| plan.is_some()),
        "at least one candidate subtype must resolve"
    );
}

/// Verifies a comparison between a dynamic element and a qualified literal.
#[test]
fn dispatches_dynamic_element_comparison() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         let index = 0\n\
         let flag = sizes[index] > 5mm\n",
    );
    let flag = binding_initializer(&program, "flag");
    let TypedExpressionKind::DynamicComparison { dispatch, .. } = &flag.kind else {
        panic!("expected a dynamic comparison");
    };
    assert!(dispatch.left_width > 1);
    assert_eq!(dispatch.right_width, 1);
}

/// Verifies negating a dynamic element mirrors the stored subtype.
#[test]
fn dispatches_dynamic_element_negation() {
    let program =
        compile_source("let sizes: int[] = [10mm]\nlet index = 0\nlet negative = -sizes[index]\n");
    let negative = binding_initializer(&program, "negative");
    let TypedExpressionKind::DynamicBinary { dispatch, .. } = &negative.kind else {
        panic!("expected a dynamic negation");
    };
    assert_eq!(dispatch.left_width, 1);
    assert!(dispatch.right_width > 1);
}

/// Verifies two dynamic element operands dispatch over both subtype slots.
#[test]
fn dispatches_two_dynamic_element_operands() {
    let program = compile_source(
        "let left: int[] = [10mm]\n\
         let right: int[] = [2cm]\n\
         let index = 0\n\
         let total = left[index] + right[index]\n",
    );
    let total = binding_initializer(&program, "total");
    let TypedExpressionKind::DynamicBinary { dispatch, .. } = &total.kind else {
        panic!("expected a dynamic binary operation");
    };
    assert!(dispatch.left_width > 1);
    assert!(dispatch.right_width > 1);
}

/// Verifies a literal the declared element type cannot represent is rejected.
#[test]
fn rejects_inexact_literal_element_conversion() {
    let error = compile_error("let sizes: int<cm>[] = [5mm]\n");
    assert!(
        error.message.contains("cannot be represented"),
        "unexpected diagnostic: {}",
        error.message
    );
}

/// Verifies an exactly convertible literal element still compiles.
#[test]
fn accepts_exact_literal_element_conversion() {
    let program = compile_source("let sizes: int<cm>[] = [10mm]\n");
    assert!(first_array_type(&program).element.subtype.is_some());
}

/// Verifies a conversion of a dynamic element pre-resolves one plan per subtype.
#[test]
fn dispatches_dynamic_element_conversion() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         let index = 1\n\
         let millimeters = sizes[index]->to(mm)\n",
    );
    let millimeters = binding_initializer(&program, "millimeters");
    let TypedExpressionKind::DynamicConvert { dispatch, .. } = &millimeters.kind else {
        panic!("expected a dynamic conversion");
    };
    assert!(dispatch.plans.len() > 1);
    assert!(
        dispatch.plans.iter().any(|plan| plan.is_some()),
        "at least one candidate source subtype must convert"
    );
}

/// Verifies a conversion after a dynamic write still dispatches on the subtype.
#[test]
fn dispatches_dynamic_conversion_after_dynamic_write() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         let index = 1\n\
         sizes[index] = 3dm\n\
         let millimeters = sizes[index]->to(mm)\n",
    );
    let millimeters = binding_initializer(&program, "millimeters");
    assert!(
        matches!(millimeters.kind, TypedExpressionKind::DynamicConvert { .. }),
        "the converted read must dispatch on the stored subtype"
    );
}

/// Verifies a conversion compiles when at least one candidate subtype converts.
///
/// The table cannot know which subtypes a program will store, so a target that
/// some registered source subtype converts to stays compilable; the runtime
/// reports the candidate that has no conversion only when it is reached.
#[test]
fn compiles_dynamic_conversion_with_a_partial_candidate_set() {
    let program = compile_source(
        "let sizes: int[] = [10mm]\n\
         let index = 0\n\
         let mass = sizes[index]->to(kg)\n",
    );
    let mass = binding_initializer(&program, "mass");
    let TypedExpressionKind::DynamicConvert { dispatch, .. } = &mass.kind else {
        panic!("expected a dynamic conversion");
    };
    assert!(dispatch.plans.iter().any(|plan| plan.is_some()));
    assert!(
        dispatch.plans.iter().any(|plan| plan.is_none()),
        "subtypes with no conversion to the target must stay unresolved"
    );
}

/// Verifies assigning a dynamic element keeps the target binding dynamic.
#[test]
fn keeps_dynamic_type_after_assignment() {
    let program = compile_source(
        "let sizes: int[] = [2cm]\n\
         let index = 0\n\
         let total: int = 0\n\
         total = sizes[index]\n\
         let sum = total + 1mm\n",
    );
    let sum = binding_initializer(&program, "sum");
    assert!(
        matches!(sum.kind, TypedExpressionKind::DynamicBinary { .. }),
        "an assigned dynamic value must keep dispatching later"
    );
}

/// Verifies a repeated element read in a `for` body becomes dynamic when the
/// body can change the element, because the emitted body must stay valid while
/// the loop runs more than once.
#[test]
fn refines_loop_head_for_a_for_body_that_writes_the_element() {
    let program = compile_source(
        "let a: int[] = [10mm]\n\
         for (i in 0..2) {\n\
         let read = a[0] + 1mm\n\
         a[0] = 2cm\n\
         }\n",
    );
    let read = loop_body_initializer(&program, "read");
    assert!(
        matches!(read.kind, TypedExpressionKind::DynamicBinary { .. }),
        "a repeated body that writes the element cannot keep a static element type"
    );
}

/// Verifies refining the loop head also reaches the loop condition, so a
/// `while` condition stops trusting the subtype an earlier iteration replaced.
#[test]
fn refines_loop_head_for_a_while_condition() {
    let program = compile_source(
        "let a: int[] = [10mm]\n\
         let index = 0\n\
         a[index] = 2cm\n\
         while (a[0] < 15mm) {\n\
         a[0] = 2cm\n\
         }\n",
    );
    let TypedStatement::While { condition, .. } = compiled_while_statement(&program) else {
        panic!("the program declares a while loop");
    };
    assert!(
        matches!(
            condition.kind,
            TypedExpressionKind::DynamicComparison { .. }
        ),
        "a loop condition that reads a mutated element must dispatch at runtime"
    );
}

/// Verifies a loop that never writes an element keeps its precise subtype, so a
/// repeated read is not silently degraded to subtype dispatch.
#[test]
fn keeps_a_stable_element_precise_inside_a_loop() {
    let program = compile_source(
        "let a: int[] = [10mm]\n\
         for (i in 0..100) {\n\
         let read = a[0] + 1mm\n\
         }\n",
    );
    let read = loop_body_initializer(&program, "read");
    assert!(
        matches!(read.kind, TypedExpressionKind::Binary { .. }),
        "a loop that never writes the element must keep the static element type"
    );
}

/// Verifies writing one slot leaves an independently known slot precise, because
/// constant indexing lets the flow state track elements separately.
#[test]
fn keeps_an_unwritten_slot_precise_when_another_slot_is_written() {
    let program = compile_source(
        "let a = [10mm, 20cm]\n\
         for (i in 0..100) {\n\
         a[1] = 30cm\n\
         let read = a[0] + 1mm\n\
         }\n",
    );
    let read = loop_body_initializer(&program, "read");
    assert!(
        matches!(read.kind, TypedExpressionKind::Binary { .. }),
        "an untouched slot must stay precise when another slot is written"
    );
}

/// Verifies an assignment written after a `break` cannot reach the loop's post
/// state, so the value carried out of the loop is the one written before it.
#[test]
fn unreachable_write_after_a_break_does_not_reach_the_loop_exit() {
    let program = compile_source(
        "let a: int[] = [10mm]\n\
         for (i in 0..1) {\n\
         a[0] = 2cm\n\
         break\n\
         a[0] = 10mm\n\
         }\n\
         let total = a[0] + 1mm\n",
    );
    let total = binding_initializer(&program, "total");
    assert!(
        matches!(total.kind, TypedExpressionKind::DynamicBinary { .. }),
        "the exit state must be the merge of the break and the iterations that reach it"
    );
}

/// Verifies an assignment written after a `continue` cannot reach the loop head,
/// so a later read in the body still sees the merged possibilities.
#[test]
fn unreachable_write_after_a_continue_does_not_reach_the_loop() {
    let program = compile_source(
        "let a: int[] = [10mm]\n\
         for (i in 0..2) {\n\
         if (i == 0) {\n\
         a[0] = 2cm\n\
         continue\n\
         a[0] = 10mm\n\
         }\n\
         let read = a[0] + 1mm\n\
         }\n",
    );
    let read = loop_body_initializer(&program, "read");
    assert!(
        matches!(read.kind, TypedExpressionKind::DynamicBinary { .. }),
        "the merged head must not trust the unreachable write"
    );
}

/// Returns the first `while` statement of a compiled program.
fn compiled_while_statement(program: &TypedProgram) -> &TypedStatement {
    program
        .statements
        .iter()
        .find(|statement| matches!(statement, TypedStatement::While { .. }))
        .expect("program declares a while loop")
}

/// Returns the initializer of one binding declared inside a loop body.
fn loop_body_initializer<'program>(
    program: &'program TypedProgram,
    name: &str,
) -> &'program TypedExpression {
    for statement in &program.statements {
        let body = match statement {
            TypedStatement::For { body, .. } | TypedStatement::While { body, .. } => body,
            _ => continue,
        };
        for nested in &body.statements {
            if let TypedStatement::VariableDeclaration {
                name: binding,
                expression,
                ..
            } = nested
                && binding == name
            {
                return expression;
            }
        }
    }
    panic!("no loop body declares a binding `{name}`");
}
