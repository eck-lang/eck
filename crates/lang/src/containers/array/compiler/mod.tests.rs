//! Tests for array compilation as one module.
//!
//! These tests compile whole array programs and inspect the IR they produce, so
//! they exercise the child modules of array compilation together rather than one
//! of them in isolation. That is the same role `lib.tests.rs` plays for a crate
//! façade, and it is why this file is named for the module it verifies.

use crate::measures::MeasuresExtension;
use crate::semantic::{
    ArrayElementContract, ArrayEndOperation, ArrayType, Extension, Registry, ScalarRepresentation,
    SemanticType, ValueType,
};

use crate::ir::{TypedExpression, TypedExpressionKind, TypedProgram};
use crate::{CompileError, TypedStatement, compile};

/// Builds a registry with every primitive and measure type for array tests.
///
/// The measure extension supplies the qualified subtypes and bidirectional
/// conversions the constrained-element tests rely on, so the tests exercise the
/// same registry contracts the CLI dialect installs.
fn array_registry() -> Registry {
    let mut registry = Registry::new();
    crate::primitives::register_all(&mut registry).unwrap();
    MeasuresExtension.register(&mut registry).unwrap();
    registry
}

/// Compiles one source program or panics with the reported diagnostic.
fn compile_source(source: &str) -> TypedProgram {
    let program = crate::parser::parse(source).expect("source must parse");
    compile(&program, &array_registry()).expect("source must compile")
}

/// Compiles one source program against a caller-owned registry.
fn compile_with_registry(source: &str, registry: &Registry) -> TypedProgram {
    let program = crate::parser::parse(source).expect("source must parse");
    compile(&program, registry).expect("source must compile")
}

/// Returns the compiler diagnostic produced by one rejected source program.
fn compile_error(source: &str) -> CompileError {
    let program = crate::parser::parse(source).expect("source must parse");
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
                match expression
                    .output
                    .as_ref()
                    .expect("element access produces a value")
                {
                    SemanticType::Scalar(value_type) => *value_type,
                    SemanticType::Open
                    | SemanticType::Array(_)
                    | SemanticType::Map(_)
                    | SemanticType::Union(_) => {
                        panic!("element access cannot produce a container or union")
                    }
                },
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
    let program = crate::parser::parse("let sizes: int<mm>[] = [10mm, 20mm, 30mm]\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let array_type = first_array_type(&program);
    assert_eq!(
        array_type
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .subtype,
        Some(millimeter)
    );
    assert_eq!(
        array_type.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );
}

/// Verifies structured array annotations preserve adaptive, qualified, and fixed contracts.
#[test]
fn resolves_structured_array_type_annotations() {
    let adaptive = first_array_type(&compile_source("let values: int[] = []\n"));
    assert_eq!(
        adaptive.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );

    let qualified = first_array_type(&compile_source("let values: int<mm>[] = []\n"));
    assert!(
        qualified
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .subtype
            .is_some()
    );
    assert_eq!(
        qualified.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );

    let fixed = first_array_type(&compile_source("let values: int64<mm>[] = []\n"));
    assert!(
        fixed
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .subtype
            .is_some()
    );
    assert_eq!(
        fixed.static_representation(),
        Some(ScalarRepresentation::Exact)
    );
}

/// Verifies nullable array annotations lower to an array-or-null union.
#[test]
fn accepts_nullable_structured_array_annotations() {
    let program = compile_source("let values: int<mm>[]? = []\n");
    assert!(matches!(
        program
            .bindings
            .first()
            .map(|binding| &binding.semantic_type),
        Some(SemanticType::Union(_))
    ));
}

/// Verifies `int64[]` is fixed while `int[]` is adaptive.
#[test]
fn distinguishes_adaptive_int_from_fixed_width() {
    let registry = array_registry();
    let adaptive_program = crate::parser::parse("let values: int[] = [1]\n").unwrap();
    let fixed_program = crate::parser::parse("let values: int64[] = [1]\n").unwrap();
    let adaptive = first_array_type(&compile(&adaptive_program, &registry).unwrap());
    let fixed = first_array_type(&compile(&fixed_program, &registry).unwrap());
    assert_eq!(
        adaptive.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );
    assert_eq!(
        fixed.static_representation(),
        Some(ScalarRepresentation::Exact)
    );
    assert_eq!(
        adaptive
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .base,
        fixed
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .base
    );
}

/// Verifies an unannotated literal creates a dynamic element contract.
#[test]
fn infers_identical_complete_element_type() {
    let program = compile_source("let sizes = [10mm, 20mm, 30mm]\n");
    let array_type = first_array_type(&program);
    assert!(matches!(
        array_type.element_contract,
        ArrayElementContract::Dynamic
    ));
}

/// Verifies differing subtypes remain concrete values in a dynamic array.
#[test]
fn infers_common_base_for_differing_subtypes() {
    let program = compile_source("let sizes = [10mm, 2cm, 3dm]\n");
    let array_type = first_array_type(&program);
    assert!(matches!(
        array_type.element_contract,
        ArrayElementContract::Dynamic
    ));
}

/// Verifies an untyped integer literal array is dynamically extensible.
#[test]
fn infers_adaptive_integer_elements() {
    let program = compile_source("let numbers = [1, 2, 3]\n");
    let array_type = first_array_type(&program);
    assert!(matches!(
        array_type.element_contract,
        ArrayElementContract::Dynamic
    ));
}

/// Verifies an empty literal needs no element type contract.
#[test]
fn accepts_empty_array_without_annotation() {
    let program = compile_source("let values = []\nvalues->push(1)\nvalues->push('one')\n");
    assert!(matches!(
        first_array_type(&program).element_contract,
        ArrayElementContract::Dynamic
    ));
}

/// Verifies an element with no finite observations lowers to open dispatch.
#[test]
fn compiles_open_dispatch_for_an_unobserved_dynamic_element() {
    let program = compile_source("let values = []\nvalues[0] + 1\n-values[0]\nvalues[0] < 1\n");
    let TypedStatement::Expression(expression) = &program.statements[1] else {
        panic!("expected expression statement");
    };
    assert!(matches!(
        expression.kind,
        TypedExpressionKind::OpenBinary { .. }
    ));
    let TypedStatement::Expression(expression) = &program.statements[2] else {
        panic!("expected expression statement");
    };
    assert!(matches!(
        expression.kind,
        TypedExpressionKind::OpenNegation { .. }
    ));
    let TypedStatement::Expression(expression) = &program.statements[3] else {
        panic!("expected expression statement");
    };
    assert!(matches!(
        expression.kind,
        TypedExpressionKind::OpenComparison { .. }
    ));
}

/// Verifies a typed empty array compiles from its declared element contract.
#[test]
fn accepts_typed_empty_array() {
    let program = compile_source("let values: int[] = []\n");
    assert_eq!(
        first_array_type(&program)
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .subtype,
        None
    );
}

/// Verifies dynamic array elements may have unrelated concrete types.
#[test]
fn accepts_elements_with_unrelated_bases() {
    compile_source("let values = [10mm, \"hello\", true, null]\n");
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
        matches!(
            first.semantic_type,
            SemanticType::Scalar(ValueType {
                subtype: Some(_),
                ..
            })
        ),
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
        match read.semantic_type {
            SemanticType::Scalar(value_type) => value_type.subtype,
            SemanticType::Open
            | SemanticType::Array(_)
            | SemanticType::Map(_)
            | SemanticType::Union(_) => {
                panic!("an element read cannot be a container or union")
            }
        },
        None,
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

/// Verifies nested arrays are ordinary values in an outer dynamic array.
#[test]
fn compiles_nested_array_literals() {
    let program = compile_source("let values = [[1, 2], [3, 4]]\n");
    let outer = first_array_type(&program);
    assert!(matches!(
        outer.element_contract,
        ArrayElementContract::Dynamic
    ));
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
    let output = expression
        .output
        .clone()
        .expect("the element produces a value");
    let output = match output {
        SemanticType::Scalar(value_type) => value_type,
        SemanticType::Open
        | SemanticType::Array(_)
        | SemanticType::Map(_)
        | SemanticType::Union(_) => {
            panic!("an element store cannot produce a container or union")
        }
    };
    assert_ne!(
        output.base,
        first_array_type(&program)
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .base
    );
}

/// Verifies a fixed-width store carries the declared element contract.
///
/// Reading an `int8` element and adding to it may promote the result while it is
/// evaluated, so the store must carry the destination the value has to satisfy.
#[test]
fn embeds_the_declared_contract_in_a_fixed_width_store() {
    let registry = array_registry();
    let int8 = registry.type_by_name("int8").expect("int8 is registered");
    let program =
        crate::parser::parse("let values: int8[] = [127]\nvalues[0] = values[0] + 1\n").unwrap();
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
    assert_eq!(
        expression.output,
        Some(SemanticType::Scalar(ValueType::plain(int8)))
    );
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
    let program =
        crate::parser::parse("let values: int[] = [1]\nvalues[0] = values[0] + 1\n").unwrap();
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
        crate::parser::parse("let sizes: int8<mm>[] = [10mm]\nsizes[0] = sizes[0] + 1mm\n")
            .unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = first_indexed_assignment_element(&program);
    let TypedExpressionKind::ElementStore { element, .. } = &element.kind else {
        panic!("a fixed-width subtype store must carry its destination contract");
    };
    assert_eq!(*element, ValueType::qualified(int8, millimeter));
}

/// Verifies an unannotated array does not inherit a value's static contract.
#[test]
fn checks_an_inferred_fixed_width_array_element() {
    let program = compile_source("let x: int8 = 127\nlet y: int8 = 1\nlet values = [x + y]\n");
    let elements = array_literal_elements(binding_initializer(&program, "values"));
    assert!(!matches!(
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
            type_domain: Some(_),
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
            type_domain: Some(_),
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
        dispatch.left_domain.len() > 1,
        "the dynamic operand contributes one slot per complete identity"
    );
    assert_eq!(dispatch.right_domain.len(), 1);
    assert!(
        dispatch.plans.iter().any(|plan| plan.is_some()),
        "at least one candidate subtype must resolve"
    );
}

/// Verifies adaptive dispatch covers complete signed bases and overflow results.
#[test]
fn dynamic_adaptive_dispatch_covers_complete_signed_bases_and_promotions() {
    let registry = array_registry();
    let program = compile_with_registry(
        "let values: int[] = [1]\n\
         let index = 0\n\
         let total = values[index] + 1\n",
        &registry,
    );
    let total = binding_initializer(&program, "total");
    let TypedExpressionKind::DynamicBinary { dispatch, .. } = &total.kind else {
        panic!("expected a dynamic binary operation");
    };
    let expected_bases = ["int8", "int16", "int32", "int64", "int128", "bigint"];
    for name in expected_bases {
        let base = registry.type_by_name(name).expect("integer is registered");
        assert!(
            dispatch
                .left_domain
                .candidates
                .iter()
                .any(|candidate| candidate == &ValueType::plain(base)),
            "adaptive dispatch is missing plain `{name}`"
        );
    }
    let result_domain = dispatch
        .result_domain
        .as_ref()
        .expect("arithmetic has promoted result identities");
    for name in ["int64", "int128", "bigint"] {
        let base = registry.type_by_name(name).expect("integer is registered");
        assert!(
            result_domain
                .candidates
                .iter()
                .any(|candidate| candidate == &ValueType::plain(base)),
            "result domain is missing `{name}` promotion"
        );
    }
}

/// Verifies an exact-width array keeps its declared base in dynamic plans.
#[test]
fn fixed_width_dynamic_dispatch_keeps_the_declared_base() {
    let registry = array_registry();
    let program = compile_with_registry(
        "let values: int64[] = [1]\n\
         let index = 0\n\
         let total = values[index] + 1\n",
        &registry,
    );
    let total = binding_initializer(&program, "total");
    let TypedExpressionKind::DynamicBinary { dispatch, .. } = &total.kind else {
        panic!("expected a dynamic binary operation");
    };
    let int64 = registry.type_by_name("int64").expect("int64 is registered");
    assert!(!dispatch.left_domain.is_empty());
    assert!(
        dispatch
            .left_domain
            .candidates
            .iter()
            .all(|candidate| candidate.base == int64),
        "an exact array must not dispatch as int128 or bigint"
    );
}

/// Verifies dynamic index plans include extractors for int128 and bigint.
#[test]
fn dynamic_index_dispatch_resolves_wide_integer_extractors() {
    let registry = array_registry();
    let program = compile_with_registry(
        "let values: int[] = [10]\n\
         let indexes: int[] = [1]\n\
         let selector = 0\n\
         let read = values[indexes[selector]]\n",
        &registry,
    );
    let read = binding_initializer(&program, "read");
    let TypedExpressionKind::ElementAccess {
        index_dispatch: Some(dispatch),
        ..
    } = &read.kind
    else {
        panic!("expected a dynamic index dispatch");
    };
    for name in ["int128", "bigint"] {
        let base = registry.type_by_name(name).expect("integer is registered");
        let slot = dispatch
            .domain
            .candidates
            .iter()
            .position(|candidate| *candidate == ValueType::plain(base))
            .expect("wide plain integer candidate is present");
        assert!(
            dispatch.extractors[slot].is_some(),
            "dynamic index has no extractor for `{name}`"
        );
    }
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
    assert!(dispatch.left_domain.len() > 1);
    assert_eq!(dispatch.right_domain.len(), 1);
}

/// Verifies negating a dynamic element mirrors the stored subtype.
#[test]
fn dispatches_dynamic_element_negation() {
    let program =
        compile_source("let sizes: int[] = [10mm]\nlet index = 0\nlet negative = -sizes[index]\n");
    let negative = binding_initializer(&program, "negative");
    let TypedExpressionKind::DynamicNegation { dispatch, .. } = &negative.kind else {
        panic!("expected a dynamic negation");
    };
    assert!(dispatch.domain.len() > 1);
    assert!(dispatch.plans.iter().any(|plan| plan.is_some()));
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
    assert!(dispatch.left_domain.len() > 1);
    assert!(dispatch.right_domain.len() > 1);
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
    assert!(
        first_array_type(&program)
            .static_semantic_type()
            .expect("static array element")
            .as_scalar()
            .expect("scalar element")
            .subtype
            .is_some()
    );
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

/// Returns the operation of the first compiled array method call.
fn first_array_method(program: &TypedProgram) -> ArrayEndOperation {
    for statement in &program.statements {
        if let TypedStatement::Expression(expression) = statement
            && let TypedExpressionKind::ArrayMethod { method, .. } = &expression.kind
        {
            return *method;
        }
    }
    panic!("program calls no array method");
}

/// Verifies `append` is the `push` operation rather than a second one.
#[test]
fn resolves_append_as_the_push_operation() {
    let pushed = compile_source("let values: int[] = [1]\nvalues->push(2)\n");
    let appended = compile_source("let values: int[] = [1]\nvalues->append(2)\n");

    assert_eq!(first_array_method(&pushed), ArrayEndOperation::Push);
    assert_eq!(first_array_method(&appended), ArrayEndOperation::Push);
}

/// Verifies `prepend` is the `unshift` operation rather than a second one.
#[test]
fn resolves_prepend_as_the_unshift_operation() {
    let unshifted = compile_source("let values: int[] = [1]\nvalues->unshift(2)\n");
    let prepended = compile_source("let values: int[] = [1]\nvalues->prepend(2)\n");

    assert_eq!(first_array_method(&unshifted), ArrayEndOperation::Unshift);
    assert_eq!(first_array_method(&prepended), ArrayEndOperation::Unshift);
}

/// Verifies the argument-less spelling names the same removal as `pop()`.
///
/// A bare `->name` is how ECK spells an argument-less method call, so it must
/// reach the same operation and the same arity rule as the parenthesized form.
#[test]
fn resolves_a_bare_arrow_spelling_as_the_same_removal() {
    let parenthesized = compile_source("let values: int[] = [1]\nvalues->pop()\n");
    let bare = compile_source("let values: int[] = [1]\nvalues->pop\n");
    let insertion = compile_error("let values: int[] = [1]\nvalues->push\n");

    assert_eq!(first_array_method(&parenthesized), ArrayEndOperation::Pop);
    assert_eq!(first_array_method(&bare), ArrayEndOperation::Pop);
    assert!(
        insertion
            .message
            .contains("`push` expects exactly one value to add")
    );
}

/// Verifies a name no array method claims lists the supported methods.
#[test]
fn rejects_an_unknown_array_method() {
    let error = compile_error("let values: int[] = [1]\nvalues->length()\n");

    assert!(error.message.contains("an array has no method `length`"));
    assert!(
        error
            .message
            .contains("push, append, pop, unshift, prepend, shift")
    );
}

/// Verifies an insertion requires exactly one value and a removal none.
#[test]
fn rejects_wrong_argument_counts() {
    let missing = compile_error("let values: int[] = [1]\nvalues->push()\n");
    let extra = compile_error("let values: int[] = [1]\nvalues->push(2, 3)\n");
    let removal = compile_error("let values: int[] = [1]\nvalues->pop(0)\n");

    assert!(
        missing
            .message
            .contains("`push` expects exactly one value to add")
    );
    assert!(
        extra
            .message
            .contains("`push` expects exactly one value to add")
    );
    assert!(
        removal
            .message
            .contains("`pop` removes one element and takes no arguments")
    );
}

/// Verifies an end operation requires a mutable array binding as its receiver.
#[test]
fn rejects_an_invalid_receiver() {
    let immutable = compile_error("const values: int[] = [1]\nvalues->push(2)\n");
    let temporary = compile_error("let values: int[] = [1]\n[2]->push(3)\n");

    assert!(
        immutable
            .message
            .contains("cannot call `push` through immutable binding `values`")
    );
    assert!(
        temporary
            .message
            .contains("its receiver must be an array binding")
    );
}

/// Verifies an insertion produces no value and a removal produces an element.
#[test]
fn distinguishes_insertion_and_removal_results() {
    let insertion = compile_error("let values: int[] = [1]\nlet stored = values->push(2)\n");
    let program = compile_source("let values: int[] = [1]\nlet removed = values->pop()\n");
    let element = program
        .bindings
        .first()
        .expect("the array binding is declared")
        .semantic_type
        .clone();
    let element = match element {
        SemanticType::Array(array_type) => array_type
            .static_semantic_type()
            .expect("typed array element")
            .clone(),
        SemanticType::Open
        | SemanticType::Scalar(_)
        | SemanticType::Map(_)
        | SemanticType::Union(_) => {
            panic!("an array binding must have an array semantic type")
        }
    };

    assert!(
        insertion
            .message
            .contains("a void expression cannot initialize a binding")
    );
    let output = binding_initializer(&program, "removed")
        .output
        .clone()
        .expect("removal produces a value");
    let SemanticType::Union(members) = output else {
        panic!("removal output must include the empty-array null case");
    };
    assert!(members.iter().any(|member| member == &element));
}

/// Verifies an inferred removal binding is nullable without an annotation.
///
/// The removal's type is the element type widened with null, so a declaration
/// that omits an annotation takes that nullability from its initializer.
#[test]
fn infers_a_nullable_binding_from_a_removal() {
    let program = compile_source("let values: int[] = [1]\nlet removed = values->pop()\n");
    let binding = program
        .bindings
        .iter()
        .find(|binding| binding.name == "removed")
        .expect("the removed binding is declared");

    assert!(matches!(binding.semantic_type, SemanticType::Union(_)));
}

/// Verifies a removal cannot initialize or be used as a non-null scalar.
#[test]
fn rejects_a_removal_where_a_non_null_scalar_is_required() {
    let annotated = compile_error("let values: int[] = [1]\nlet removed: int = values->pop()\n");
    let arithmetic = compile_error("let values: int[] = [1]\nlet total = values->pop() + 1\n");

    assert!(
        annotated
            .message
            .contains("nullable value cannot initialize non-nullable binding `removed`")
    );
    assert!(
        arithmetic
            .message
            .contains("nullable value must be narrowed before this operation")
    );
}

/// Verifies an insertion enforces the declared element representation.
///
/// A literal the declared width cannot hold is rejected while compiling, exactly
/// as it is for an array literal or an indexed assignment, because insertion
/// crosses the same element contract instead of a conversion path of its own.
#[test]
fn rejects_an_inserted_value_the_element_contract_cannot_hold() {
    let error = compile_error("let values: int8[] = []\nvalues->push(300)\n");

    assert!(
        error
            .message
            .contains("invalid literal `300` for type `int8`")
    );
}

/// Verifies an insertion converts a compatible element subtype.
#[test]
fn converts_an_inserted_element_to_the_declared_subtype() {
    let program = compile_source("let sizes: int<mm>[] = []\nsizes->push(2cm)\n");
    let stored = first_array_method_argument(&program);
    let output = stored
        .output
        .clone()
        .expect("the stored value produces a type");
    let element = program
        .bindings
        .first()
        .expect("the array binding is declared")
        .semantic_type
        .clone();
    let element = match element {
        SemanticType::Array(array_type) => array_type
            .static_semantic_type()
            .expect("typed array element")
            .clone(),
        SemanticType::Open
        | SemanticType::Scalar(_)
        | SemanticType::Map(_)
        | SemanticType::Union(_) => {
            panic!("an array binding must have an array semantic type")
        }
    };

    assert!(
        matches!(
            stored.kind,
            TypedExpressionKind::Convert {
                target_base: Some(_),
                ..
            }
        ),
        "a constrained element must be converted before it is stored"
    );
    assert_eq!(
        output,
        SemanticType::Scalar(element.as_scalar().expect("scalar element"))
    );
}

/// Verifies insertion preserves unaffected constant-index flow knowledge.
#[test]
fn invalidates_recorded_element_types_after_an_insertion() {
    let program = compile_source(
        "let sizes: int[] = [10mm]\n\
         sizes->push(2cm)\n\
         let first = sizes[0]\n\
         let sum = first + 1mm\n",
    );
    let sum = binding_initializer(&program, "sum");

    assert!(
        matches!(sum.kind, TypedExpressionKind::Binary { .. }),
        "insertion at the back must preserve the first element's type"
    );
}

/// Verifies removal shifts the recorded element types with their values.
#[test]
fn invalidates_recorded_element_types_after_a_removal() {
    let program = compile_source(
        "let sizes: int[] = [10mm, 2cm]\n\
         sizes->shift()\n\
         let first = sizes[0]\n\
         let sum = first + 1mm\n",
    );
    let sum = binding_initializer(&program, "sum");

    assert!(
        matches!(sum.kind, TypedExpressionKind::Binary { .. }),
        "a removal must retain the shifted element's concrete type"
    );
}

/// Returns the stored value of the first compiled array method call.
fn first_array_method_argument(program: &TypedProgram) -> &TypedExpression {
    for statement in &program.statements {
        if let TypedStatement::Expression(expression) = statement
            && let TypedExpressionKind::ArrayMethod { arguments, .. } = &expression.kind
        {
            return arguments
                .first()
                .expect("the array method stores one value");
        }
    }
    panic!("program calls no array method");
}

/// Verifies non-nullable array storage rejects nullable values while nullable
/// array contracts retain them as concrete null values.
#[test]
fn handles_nullable_values_at_every_array_storage_boundary() {
    for source in [
        "let item: int? = 1\nlet values: int[] = [item]\n",
        "let item: int? = 1\nlet values: int[] = [0]\nvalues[0] = item\n",
        "let item: int? = 1\nlet values: int[] = []\nvalues->push(item)\n",
        "let item: int? = 1\nlet values: int[] = []\nvalues->append(item)\n",
        "let item: int? = 1\nlet values: int[] = []\nvalues->unshift(item)\n",
        "let item: int? = 1\nlet values: int[] = []\nvalues->prepend(item)\n",
        "let values: int[] = [1]\nvalues->push(values->pop())\n",
        "let values: int[] = [null]\n",
    ] {
        let error = compile_error(source);
        assert_eq!(
            error.message,
            "nullable value must be narrowed before this operation"
        );
    }
    let inferred = compile_source("let item: int? = 1\nlet values = [item]\n");
    assert!(matches!(
        first_array_type(&inferred).element_contract,
        ArrayElementContract::Dynamic
    ));
    compile_source("let values: int?[] = [null, 1]\n");
}

/// Verifies a non-null proof remains valid when its value enters array storage.
#[test]
fn accepts_a_narrowed_value_at_array_storage_boundaries() {
    compile_source(
        "let item: int? = 1\n\
         if (item != null) {\n\
         let values: int[] = [item]\n\
         values[0] = item\n\
         values->push(item)\n\
         values->unshift(item)\n\
         }\n",
    );
}

/// Verifies an inexact fixed-width unit conversion is rejected before division truncates.
#[test]
fn rejects_inexact_fixed_width_unit_conversion() {
    let error = compile_error("let sizes: int8<cm>[] = [15mm]\n");
    assert!(
        error.message.contains("cannot be represented"),
        "unexpected diagnostic: {}",
        error.message
    );
}

/// Verifies signed adaptive literal widening follows the registered width order.
#[test]
fn widens_adaptive_literals_through_int128_before_bigint() {
    let registry = array_registry();
    let int128 = registry
        .type_by_name("int128")
        .expect("int128 is registered");
    let program = crate::parser::parse("let values: int[] = [9223372036854775808]\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = array_literal_elements(binding_initializer(&program, "values"))[0]
        .output
        .clone()
        .expect("the literal produces a value");
    assert_eq!(element, SemanticType::Scalar(ValueType::plain(int128)));
}

/// Verifies adaptive widening reaches bigint only after int128 is exhausted.
#[test]
fn widens_adaptive_literals_to_bigint_after_int128() {
    let registry = array_registry();
    let bigint = registry
        .type_by_name("bigint")
        .expect("bigint is registered");
    let program =
        crate::parser::parse("let values: int[] = [170141183460469231731687303715884105728]\n")
            .unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = array_literal_elements(binding_initializer(&program, "values"))[0]
        .output
        .clone()
        .expect("the literal produces a value");
    assert_eq!(element, SemanticType::Scalar(ValueType::plain(bigint)));
}

/// Verifies negative literals use the signed token when adaptive widening is needed.
#[test]
fn widens_negative_adaptive_literals_as_signed_values() {
    let registry = array_registry();
    let int128 = registry
        .type_by_name("int128")
        .expect("int128 is registered");
    let program = crate::parser::parse("let values: int[] = [-9223372036854775809]\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = array_literal_elements(binding_initializer(&program, "values"))[0]
        .output
        .clone()
        .expect("the literal produces a value");
    assert_eq!(element, SemanticType::Scalar(ValueType::plain(int128)));
}

/// Verifies the parser-shaped signed minimum is accepted by fixed-width storage.
#[test]
fn accepts_a_signed_minimum_in_fixed_width_array_storage() {
    let registry = array_registry();
    let int8 = registry.type_by_name("int8").expect("int8 is registered");
    let source = crate::parser::parse("let values: int8[] = [-128]\n").unwrap();
    let program = compile(&source, &registry).unwrap();
    let element = array_literal_elements(binding_initializer(&program, "values"))[0]
        .output
        .clone()
        .expect("the literal produces a value");
    assert_eq!(element, SemanticType::Scalar(ValueType::plain(int8)));
}

/// Verifies constrained adaptive literals keep their widened signed representation.
#[test]
fn widens_constrained_adaptive_literals_without_forcing_int64() {
    let registry = array_registry();
    let int128 = registry
        .type_by_name("int128")
        .expect("int128 is registered");
    let millimeter = registry.subtype_by_suffix("mm").expect("mm is registered");
    let program =
        crate::parser::parse("let values: int<mm>[] = [9223372036854775808mm]\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let element = array_literal_elements(binding_initializer(&program, "values"))[0]
        .output
        .clone()
        .expect("the converted literal produces a value");
    assert_eq!(
        element,
        SemanticType::Scalar(ValueType::qualified(int128, millimeter))
    );
}

/// Verifies dynamic arrays preserve widened literal identities.
#[test]
fn infers_adaptive_storage_for_widened_signed_literals() {
    let registry = array_registry();
    let int128 = registry
        .type_by_name("int128")
        .expect("int128 is registered");
    let program =
        crate::parser::parse("let values = [1, 999999999999999999999999999999999]\n").unwrap();
    let program = compile(&program, &registry).unwrap();
    let array_type = first_array_type(&program);
    assert!(matches!(
        array_type.element_contract,
        ArrayElementContract::Dynamic
    ));
    let element = array_literal_elements(binding_initializer(&program, "values"))[1]
        .output
        .clone()
        .expect("the widened literal produces a value");
    assert_eq!(element, SemanticType::Scalar(ValueType::plain(int128)));
}

/// Verifies a fixed-width source does not constrain an unannotated array.
#[test]
fn keeps_fixed_integer_sources_exact_when_inferring_an_array() {
    compile_source(
        "let source: int64 = 1\n\
         let values = [source]\n\
         values[0] = 9223372036854775808\n",
    );
}

/// Verifies an explicit adaptive source can enter a dynamic array.
#[test]
fn keeps_adaptive_integer_sources_adaptive_when_inferring_an_array() {
    compile_source(
        "let source: int = 1\n\
         let values = [source]\n\
         values[0] = 9223372036854775808\n",
    );
}

/// Verifies unsigned values do not cross into an adaptive signed integer array.
#[test]
fn rejects_unsigned_values_in_adaptive_integer_storage() {
    let error = compile_error("let item: uint8 = 1\nlet values: int[] = [item]\n");
    assert!(error.message.contains("type mismatch"));
}

/// Verifies aliases compose with unions and recursive array declarations.
#[test]
fn resolves_alias_composition_and_representation_policies() {
    let program = compile_source(
        "type Number = int | decimal\n\
         type Row = Number[]\n\
         type Matrix = Row[]\n\
         type IntegerFamily = int | int64\n\
         type Exact = int64\n\
         let value: Number = 10\n\
         let row: Row = [1, 2]\n\
         let matrix: Matrix = [[1, 2]]\n\
         let integer_family: IntegerFamily[] = [9223372036854775808]\n\
         let adaptive: Number[] = []\n\
         let fixed: Exact[] = []\n",
    );

    let value = program
        .bindings
        .iter()
        .find(|binding| binding.name == "value")
        .expect("union binding is recorded");
    assert!(matches!(value.semantic_type, SemanticType::Union(_)));

    let row = program
        .bindings
        .iter()
        .find(|binding| binding.name == "row")
        .expect("row binding is recorded");
    let SemanticType::Array(row_type) = &row.semantic_type else {
        panic!("row alias must resolve to an array");
    };
    assert_eq!(
        row_type.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );

    let matrix = program
        .bindings
        .iter()
        .find(|binding| binding.name == "matrix")
        .expect("matrix binding is recorded");
    let SemanticType::Array(matrix_type) = &matrix.semantic_type else {
        panic!("matrix alias must resolve to an array");
    };
    assert_eq!(
        matrix_type.static_representation(),
        Some(ScalarRepresentation::Exact)
    );
    assert!(matches!(
        matrix_type.static_semantic_type(),
        Some(SemanticType::Array(_))
    ));

    let adaptive = first_array_type(&program);
    assert_eq!(
        adaptive.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );
    let fixed = program
        .bindings
        .iter()
        .find(|binding| binding.name == "fixed")
        .expect("fixed binding is recorded");
    let SemanticType::Array(fixed_type) = &fixed.semantic_type else {
        panic!("fixed alias must resolve to an array");
    };
    assert_eq!(
        fixed_type.static_representation(),
        Some(ScalarRepresentation::Exact)
    );
}

/// Verifies inline unions, union-element arrays, and unions of homogeneous arrays.
#[test]
fn compiles_union_arrays_without_changing_their_shape() {
    let program = compile_source(
        "let elements: (int | string)[] = [1, 'one']\n\
         let arrays: int[] | string[] = [1, 2]\n\
         let wide: int128 = 1\n\
         let adaptive: (int | string)[] = [wide]\n\
         let widened_literal: (int | string)[] = [9223372036854775808]\n",
    );
    let elements = program
        .bindings
        .iter()
        .find(|binding| binding.name == "elements")
        .expect("union-element array is recorded");
    let SemanticType::Array(elements_type) = &elements.semantic_type else {
        panic!("union-element declaration must remain one array");
    };
    assert!(matches!(
        elements_type.static_semantic_type(),
        Some(SemanticType::Union(_))
    ));

    let arrays = program
        .bindings
        .iter()
        .find(|binding| binding.name == "arrays")
        .expect("array union is recorded");
    assert!(matches!(arrays.semantic_type, SemanticType::Union(_)));
    let adaptive = program
        .bindings
        .iter()
        .find(|binding| binding.name == "adaptive")
        .expect("adaptive union-element array is recorded");
    let SemanticType::Array(adaptive_type) = &adaptive.semantic_type else {
        panic!("adaptive union-element declaration must remain one array");
    };
    assert_eq!(
        adaptive_type.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );
    let widened_literal = program
        .bindings
        .iter()
        .find(|binding| binding.name == "widened_literal")
        .expect("adaptive union literal is recorded");
    let SemanticType::Array(widened_type) = &widened_literal.semantic_type else {
        panic!("adaptive literal declaration must remain one array");
    };
    assert_eq!(
        widened_type.static_representation(),
        Some(ScalarRepresentation::AdaptiveSignedInteger)
    );
    let rejected = compile_error(
        "let ints: int[] = [1, 2]\n\
         let values: (int | string)[] = ints\n",
    );
    assert!(rejected.message.contains("type mismatch"));

    let rejected = compile_error(
        "let wide: int128 = 1\n\
         let values: (int64 | string)[] = [wide]\n",
    );
    assert!(rejected.message.contains("array element does not satisfy"));
}

/// Verifies nullable lowering at scalar, array, and array-element positions.
#[test]
fn lowers_nested_nullable_array_shapes() {
    let program = compile_source(
        "let maybe_values: int[]? = null\n\
         let nullable_elements: int?[] = [null, 1]\n\
         let nested: int?[]? = [null, 1]\n",
    );
    assert!(program.bindings.iter().all(|binding| {
        binding.name == "nullable_elements"
            || binding.name == "nested"
            || matches!(binding.semantic_type, SemanticType::Union(_))
    }));
    assert!(matches!(
        program
            .bindings
            .iter()
            .find(|binding| binding.name == "nullable_elements")
            .map(|binding| &binding.semantic_type),
        Some(SemanticType::Array(_))
    ));
    assert!(matches!(
        program
            .bindings
            .iter()
            .find(|binding| binding.name == "nested")
            .map(|binding| &binding.semantic_type),
        Some(SemanticType::Union(_))
    ));
}

/// Verifies alias cycles are diagnosed without recursive resolution.
#[test]
fn rejects_structural_alias_cycles() {
    let error = compile_error("type A = B\ntype B = A\nlet value: A = 1\n");
    assert!(
        error.message.contains("type alias cycle: A -> B -> A"),
        "unexpected diagnostic: {}",
        error.message
    );

    let error = compile_error("type A = A[]\nlet value: A = []\n");
    assert!(error.message.contains("type alias cycle: A -> A"));

    let error = compile_error("type A = A[]\n");
    assert!(error.message.contains("type alias cycle: A -> A"));
}

/// Verifies aliases can resolve declarations that appear later in the program.
#[test]
fn resolves_forward_structural_aliases() {
    compile_source(
        "type Row = Number[]\n\
         type Number = int | decimal\n\
         let row: Row = [1, 2]\n",
    );
}

/// Verifies invariant-array diagnostics render ECK structural type syntax.
#[test]
fn renders_structural_array_assignment_diagnostics() {
    let error = compile_error(
        "let ints: int[] = [1, 2]\n\
         let values: (int | string)[] = ints\n",
    );

    assert!(
        error.message.contains("int64[]") && error.message.contains("(int64 | string)[]"),
        "unexpected diagnostic: {}",
        error.message
    );
}

/// Verifies scalar unions accept each member and reject unrelated identities.
#[test]
fn enforces_inline_union_membership() {
    compile_source(
        "let integer: int | string = 1\n\
         let text: int | string = 'one'\n",
    );

    let error = compile_error("let value: int | string = true\n");
    assert!(
        error.message.contains("bool")
            && error.message.contains("int64")
            && error.message.contains("string"),
        "unexpected diagnostic: {}",
        error.message
    );
}

/// Verifies union-element arrays accept each member at initialization and mutation.
#[test]
fn mutates_union_element_arrays_with_each_member() {
    compile_source(
        "let values: (int | string)[] = [1]\n\
         values[0] = 'one'\n",
    );
}

/// Verifies union-element arrays reject values outside their element contract.
#[test]
fn rejects_non_member_union_element_mutation() {
    let error = compile_error(
        "let values: (int | string)[] = [1]\n\
         values[0] = true\n",
    );
    assert_eq!(
        error.message,
        "array element does not satisfy its recursive array contract"
    );
}

/// Verifies source unions assign only to destinations containing every member.
#[test]
fn assigns_union_subsets_but_rejects_union_supersets() {
    compile_source(
        "let source: int | string = 1\n\
         let destination: int | string | decimal = source\n",
    );

    let error = compile_error(
        "let source: int | string | decimal = 1\n\
         let destination: int | string = source\n",
    );
    assert!(error.message.contains("type mismatch"));
}

/// Verifies duplicate and unknown aliases receive stable declaration diagnostics.
#[test]
fn rejects_duplicate_and_unknown_structural_aliases() {
    let duplicate = compile_error("type Value = int\ntype Value = string\n");
    assert!(
        duplicate
            .message
            .contains("type alias `Value` is already declared")
    );

    let unknown = compile_error("type Value = Missing\n");
    assert!(unknown.message.contains("unknown type `Missing`"));
}
