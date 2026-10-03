use super::*;
use crate::ir::{LocalVariableSlot, TypedStatement};
use crate::semantic::{
    CoreError, ExecutionContext, FunctionDeterminism, FunctionEffectSummary,
    FunctionExternalEffect, FunctionPurity, FunctionSignature, Value,
};

/// Compiles source through the real resolver, then inspects the owned side table.
fn source_analysis(source: &str) -> (TypedProgram, ExecutionAnalysis) {
    let registry = crate::semantic::default_registry().unwrap();
    let program = crate::compile(&crate::parse(source).unwrap(), &registry).unwrap();
    let analysis = analyze(&program, &registry);
    (program, analysis)
}

/// Returns the single loop entry in a focused source fixture.
fn only_loop(analysis: &ExecutionAnalysis) -> &LoopAnalysis {
    assert_eq!(analysis.loops.len(), 1);
    analysis.loops.values().next().unwrap()
}

/// Requires an independent decision and returns its snapshot/journal slot lists.
fn independent(analysis: &ExecutionAnalysis) -> (&[LocalVariableSlot], &[LocalVariableSlot]) {
    match &only_loop(analysis).parallelism {
        Parallelism::IndependentIterations {
            written_array_slots,
            capture_read_slots,
        } => (written_array_slots, capture_read_slots),
        other => panic!("expected independent iterations: {other:?}"),
    }
}

/// Requires a structured sequential decision without parsing diagnostics.
fn sequential(analysis: &ExecutionAnalysis) -> &[SequentialReason] {
    match &only_loop(analysis).parallelism {
        Parallelism::Sequential { reasons } => reasons,
        other => panic!("expected sequential iterations: {other:?}"),
    }
}

/// Returns the resolved slot of a unique fixture binding.
fn named_slot(program: &TypedProgram, name: &str) -> LocalVariableSlot {
    program
        .bindings
        .iter()
        .find(|binding| binding.name == name)
        .unwrap()
        .slot
}

/// Pure locals and immutable nested lexical blocks remain eligible.
#[test]
fn pure_computations_and_nested_blocks_are_independent() {
    let (_, analysis) = source_analysis(
        "for (i in 0..10) { let x = i * 2\n { const y = x + 10\n const z = y * y } }",
    );
    let (writes, captures) = independent(&analysis);
    assert!(writes.is_empty());
    assert!(captures.is_empty());
    assert!(analysis.blocks.iter().all(|block| block.pure));
}

/// Journal writes capture their baseline and read-only input slots exactly once.
#[test]
fn array_transform_exposes_resolved_captures_and_written_slots() {
    let (program, analysis) = source_analysis(
        "let input: int[] = [1,2,3]\nlet output: int[] = [0,0,0]\nconst offset = 2\nfor (i in 0..3) { output[i] = input[i] * 2 + offset }",
    );
    let (writes, captures) = independent(&analysis);
    assert_eq!(writes, &[named_slot(&program, "output")]);
    let mut expected = vec![
        named_slot(&program, "input"),
        named_slot(&program, "output"),
        named_slot(&program, "offset"),
    ];
    expected.sort_by_key(|slot| slot.0);
    assert_eq!(captures, expected);
    let effects = &only_loop(&analysis).effects;
    assert!(effects.may_fail);
    assert!(effects.deterministic);
    assert!(effects.external_effects.is_empty());
    assert!(only_loop(&analysis).dependencies.is_empty());
}

/// Every requested affine syntax maps to an exact injective integer expression.
#[test]
fn requested_affine_write_forms_are_independent() {
    for (index, coefficient, offset) in [
        ("i", 1, 0),
        ("i + 1", 1, 1),
        ("i - 1", 1, -1),
        ("2 * i", 2, 0),
        ("i * 2", 2, 0),
        ("2 * i + 1", 2, 1),
        ("2 * i - 1", 2, -1),
        ("5 - i", -1, 5),
    ] {
        let (_, analysis) = source_analysis(&format!(
            "let output: int[] = [0,0,0,0,0,0]\nfor (i in 0..3) {{ output[{index}] = i }}"
        ));
        assert!(
            matches!(
                only_loop(&analysis).parallelism,
                Parallelism::IndependentIterations { .. }
            ),
            "{index}: {:?}",
            only_loop(&analysis).parallelism
        );
        assert!(matches!(&only_loop(&analysis).effects.writes[0].resource,
            Resource::ArrayElement { index: IndexAccess::Affine(index), .. }
                if index.coefficient == coefficient && index.offset == offset));
    }
}

/// Different affine forms on one written slot are conservatively overlapping.
#[test]
fn overlapping_affine_forms_are_sequential() {
    for body in [
        "output[i] = output[i + 1]",
        "output[i] = i\noutput[2 * i] = i",
        "output[i] = output[2 * i]",
    ] {
        let (_, analysis) = source_analysis(&format!(
            "let output: int[] = [0,0,0,0,0,0]\nfor (i in 0..3) {{ {body} }}"
        ));
        assert!(sequential(&analysis).contains(&SequentialReason::ConflictingAccesses));
        assert!(!only_loop(&analysis).dependencies.is_empty());
    }
}

/// Multiple writes and reads at one affine element use the worker journal overlay.
#[test]
fn same_iteration_reads_and_repeated_writes_are_independent() {
    let (_, analysis) = source_analysis(
        "let output: int[] = [0,0,0]\nfor (i in 0..3) { output[i] = i\nlet first = output[i + 0]\noutput[i] = first * 2\nlet second = output[i] }",
    );
    independent(&analysis);
    assert_eq!(
        only_loop(&analysis)
            .effects
            .writes
            .iter()
            .filter(|write| matches!(write.resource, Resource::ArrayElement { .. }))
            .count(),
        2
    );
}

/// Constant, modulo, zero-coefficient, and nonlinear writes cannot prove uniqueness.
#[test]
fn colliding_and_nonlinear_writes_are_sequential() {
    for index in ["0", "i % 4", "0 * i", "i * i"] {
        let source =
            format!("let output: int[] = [0,0,0,0]\nfor (i in 0..3) {{ output[{index}] = i }}");
        let (_, analysis) = source_analysis(&source);
        assert!(
            sequential(&analysis)
                .iter()
                .any(|reason| matches!(reason, SequentialReason::NonAffineWrite(_)))
        );
        assert!(
            only_loop(&analysis)
                .dependencies
                .iter()
                .any(|dependency| dependency.kind == DependencyKind::WriteAfterWrite)
        );
    }
}

/// Captured scalar mutation retains all three loop-carried dependency directions.
#[test]
fn scalar_accumulation_retains_dependency_evidence() {
    let (_, analysis) = source_analysis("let total = 0\nfor (i in 0..10) { total = total + i }");
    assert!(
        sequential(&analysis)
            .iter()
            .any(|reason| matches!(reason, SequentialReason::CapturedMutation(_)))
    );
    for kind in [
        DependencyKind::ReadAfterWrite,
        DependencyKind::WriteAfterRead,
        DependencyKind::WriteAfterWrite,
    ] {
        assert!(
            only_loop(&analysis)
                .dependencies
                .iter()
                .any(|dependency| dependency.kind == kind)
        );
    }
}

/// A captured assignment is forbidden even when it does not read the previous value.
#[test]
fn scalar_last_value_assignment_is_sequential() {
    let (_, analysis) = source_analysis("let value = 0\nfor (i in 0..10) { value = i }");
    assert!(
        sequential(&analysis)
            .iter()
            .any(|reason| matches!(reason, SequentialReason::CapturedMutation(_)))
    );
}

/// Source-name shadowing never aliases resolved outer storage.
#[test]
fn shadowed_scalar_locals_are_worker_private() {
    let (program, analysis) =
        source_analysis("let value = 0\nfor (i in 0..10) { let value = i\nvalue = value + 2 }");
    independent(&analysis);
    let outer = named_slot(&program, "value");
    assert!(
        only_loop(&analysis)
            .effects
            .writes
            .iter()
            .all(|write| write.resource.slot() != outer)
    );
}

/// Changing the induction slot invalidates every affine access using that identity.
#[test]
fn mutated_iteration_binding_is_sequential() {
    let registry = crate::semantic::default_registry().unwrap();
    let mut program = crate::compile(
        &crate::parse("let output: int[] = [0,0,0]\nfor (i in 0..3) { output[i] = i }").unwrap(),
        &registry,
    )
    .unwrap();
    let TypedStatement::For {
        binding,
        slot,
        start,
        body,
        span,
        ..
    } = &mut program.statements[1]
    else {
        panic!();
    };
    body.statements.insert(
        0,
        TypedStatement::Assignment {
            name: "i".into(),
            binding: *binding,
            slot: *slot,
            expression: start.clone(),
            span: *span,
        },
    );
    assert!(
        sequential(&analyze(&program, &registry))
            .iter()
            .any(|reason| matches!(reason, SequentialReason::CapturedMutation(_)))
    );
}

/// COW aliases made before a loop remain stable read snapshots on distinct slots.
#[test]
fn preexisting_copy_on_write_aliases_are_independent() {
    let (_, analysis) = source_analysis(
        "let output: int[] = [1,2,3]\nconst input = output\nfor (i in 0..3) { output[i] = input[i] * 2 }",
    );
    independent(&analysis);
}

/// Aliases created inside a worker would observe a different snapshot after writes.
#[test]
fn local_aliases_of_written_arrays_are_sequential() {
    for body in [
        "const alias = output\noutput[i] = alias[i]",
        "output[i] = i\nconst alias = output",
        "const alias = output\nconst another = alias\noutput[i] = another[i]",
    ] {
        let (_, analysis) = source_analysis(&format!(
            "let output: int[] = [1,2,3]\nfor (i in 0..3) {{ {body} }}"
        ));
        assert!(
            sequential(&analysis)
                .iter()
                .any(|reason| matches!(reason, SequentialReason::WholeContainerRead(_)))
        );
    }
}

/// Whole-array function arguments are forbidden when that same slot is journaled.
#[test]
fn pure_whole_container_calls_on_written_arrays_are_sequential() {
    let mut registry = crate::semantic::default_registry().unwrap();
    registry
        .register_global_function_with_effect_summary(
            "observe",
            FunctionSignature::AnySingle,
            None,
            discard,
            FunctionEffectSummary::PURE,
        )
        .unwrap();
    let program = crate::compile(
        &crate::parse(
            "let output: int[] = [0,0,0]\nfor (i in 0..3) { output[i] = i\nobserve(output) }",
        )
        .unwrap(),
        &registry,
    )
    .unwrap();
    assert!(
        sequential(&analyze(&program, &registry))
            .iter()
            .any(|reason| matches!(reason, SequentialReason::WholeContainerRead(_)))
    );
}

/// Both branches contribute effects, with branch-local declarations staying private.
#[test]
fn compositional_branches_and_continue_are_independent() {
    let (_, analysis) = source_analysis(
        "let output: int[] = [0,0,0]\nfor (i in 0..3) { if (i == 0) { output[i] = 10\ncontinue } else { const local = i * 2\noutput[i] = local } }",
    );
    independent(&analysis);
    assert_eq!(
        only_loop(&analysis)
            .effects
            .writes
            .iter()
            .filter(|write| matches!(write.resource, Resource::ArrayElement { .. }))
            .count(),
        2
    );
}

/// A capture write in only one alternate path still rejects the whole loop.
#[test]
fn alternate_branch_capture_mutation_is_sequential() {
    let (_, analysis) = source_analysis(
        "let total = 0\nfor (i in 0..3) { if (i == 0) { const local = i } else { total = i } }",
    );
    sequential(&analysis);
}

/// Break may suppress later logical iterations, so it is never worker-local control.
#[test]
fn break_is_sequential() {
    let (_, analysis) = source_analysis("for (i in 0..3) { if (i == 1) { break } }");
    assert!(sequential(&analysis).contains(&SequentialReason::Break));
}

/// Nested loops retain an inner decision while the outer region stays sequential.
#[test]
fn nested_loop_analysis_is_inner_only() {
    let (_, analysis) =
        source_analysis("for (i in 0..3) { for (j in 0..3) { const local = i + j } }");
    assert_eq!(analysis.loops.len(), 2);
    let mut loops: Vec<_> = analysis.loops.values().collect();
    loops.sort_by_key(|analysis| analysis.span.start);
    assert!(
        matches!(&loops[0].parallelism, Parallelism::Sequential { reasons } if reasons.contains(&SequentialReason::NestedLoop))
    );
    assert!(matches!(
        &loops[1].parallelism,
        Parallelism::IndependentIterations { .. }
    ));
}

/// Mutable maps and structural array mutations are deliberately unsupported.
#[test]
fn maps_and_array_end_mutations_are_sequential() {
    for source in [
        "let values = {1: 2}\nfor (i in 0..3) { values[i] = i }",
        "let values = {1: 2}\nfor (i in 0..3) { const local = values[i] }",
        "let values: int[] = []\nfor (i in 0..3) { values->push(i) }",
        "for (i in 0..3) { let values: int[] = [0,0,0]\nvalues[i] = i }",
    ] {
        sequential(&source_analysis(source).1);
    }
}

/// While regions cannot establish independent bounded iteration scheduling.
#[test]
fn while_inside_range_is_sequential() {
    let (_, analysis) =
        source_analysis("for (i in 0..3) { while (false) { const local = i * 2 } }");
    assert!(sequential(&analysis).contains(&SequentialReason::NestedLoop));
}

/// Deterministic failures remain eligible because runtime replays the exact prefix.
#[test]
fn potentially_failing_pure_computation_is_independent() {
    let (_, analysis) =
        source_analysis("let input: int[] = [1]\nfor (i in 0..3) { const local = input[i] * 2 }");
    independent(&analysis);
    assert!(only_loop(&analysis).effects.may_fail);
}

/// Native metadata composes through arguments and nested pure calls.
#[test]
fn native_effects_are_transitive_and_structured() {
    let mut registry = crate::semantic::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    let signature = FunctionSignature::Exact(vec![integer]);
    registry
        .register_global_function_with_effect_summary(
            "identity",
            signature.clone(),
            Some(integer),
            identity,
            FunctionEffectSummary::PURE,
        )
        .unwrap();
    registry
        .register_global_function("unknown", signature.clone(), Some(integer), identity)
        .unwrap();
    for (name, summary) in [
        (
            "external_read",
            FunctionEffectSummary {
                purity: FunctionPurity::Pure,
                determinism: FunctionDeterminism::Deterministic,
                may_fail: false,
                external_effect: FunctionExternalEffect::ReadsExternalState,
            },
        ),
        (
            "nondeterministic",
            FunctionEffectSummary {
                determinism: FunctionDeterminism::Nondeterministic,
                ..FunctionEffectSummary::PURE
            },
        ),
        (
            "fallible",
            FunctionEffectSummary {
                may_fail: true,
                ..FunctionEffectSummary::PURE
            },
        ),
    ] {
        registry
            .register_global_function_with_effect_summary(
                name,
                signature.clone(),
                Some(integer),
                identity,
                summary,
            )
            .unwrap();
    }
    for (call, safe) in [
        ("identity(identity(i))", true),
        ("identity(unknown(i))", false),
        ("external_read(i)", false),
        ("nondeterministic(i)", false),
        ("fallible(i)", true),
        ("print(i)", false),
    ] {
        let program = crate::compile(
            &crate::parse(&format!("for (i in 0..3) {{ {call} }}")).unwrap(),
            &registry,
        )
        .unwrap();
        let analysis = analyze(&program, &registry);
        if safe {
            independent(&analysis);
        } else {
            sequential(&analysis);
        }
    }
}

/// Lookup uses complete byte offsets and colliding imported spans fail closed.
#[test]
fn side_table_lookup_and_duplicate_spans_are_conservative() {
    let registry = crate::semantic::default_registry().unwrap();
    let mut program = crate::compile(
        &crate::parse("for (i in 0..3) { const local = i * 2 }").unwrap(),
        &registry,
    )
    .unwrap();
    let span = match program.statements[0] {
        TypedStatement::For { span, .. } => span,
        _ => panic!(),
    };
    let analysis = analyze(&program, &registry);
    assert_eq!(analysis.loop_analysis(span).unwrap().span, span);
    assert!(
        analysis
            .loop_analysis(Span {
                start: span.start,
                end: span.end + 1
            })
            .is_none()
    );
    program.statements.push(program.statements[0].clone());
    assert!(
        sequential(&analyze(&program, &registry)).contains(&SequentialReason::DuplicateLoopSpan)
    );
}

/// Configuration changes in an iteration are observable state and force fallback.
#[test]
fn configuration_inside_iteration_is_sequential() {
    let registry = crate::semantic::default_registry().unwrap();
    let mut program = crate::compile(
        &crate::parse("for (i in 0..3) { const local = i * 2 }").unwrap(),
        &registry,
    )
    .unwrap();
    let TypedStatement::For { body, span, .. } = &mut program.statements[0] else {
        panic!();
    };
    body.statements.insert(
        0,
        TypedStatement::Configuration {
            configuration_override: crate::semantic::ConfigurationOverride::default(),
            span: *span,
        },
    );
    let analysis = analyze(&program, &registry);
    assert!(sequential(&analysis).contains(&SequentialReason::Configuration));
    assert!(
        only_loop(&analysis)
            .effects
            .external_effects
            .contains(&ExternalEffect::Configuration)
    );
}

/// Pure index callbacks do not prove that arbitrary result indices are injective.
#[test]
fn pure_function_index_is_sequential() {
    let mut registry = crate::semantic::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    registry
        .register_global_function_with_effect_summary(
            "identity",
            FunctionSignature::Exact(vec![integer]),
            Some(integer),
            identity,
            FunctionEffectSummary::PURE,
        )
        .unwrap();
    let program = crate::compile(
        &crate::parse("let output: int[] = [0,0,0]\nfor (i in 0..3) { output[identity(i)] = i }")
            .unwrap(),
        &registry,
    )
    .unwrap();
    assert!(
        sequential(&analyze(&program, &registry))
            .iter()
            .any(|reason| matches!(reason, SequentialReason::NonAffineWrite(_)))
    );
}

/// Fixed-width storage normalization is allowed only through verified builtin hooks.
#[test]
fn builtin_fixed_width_element_store_is_independent() {
    let (_, analysis) =
        source_analysis("let output: int8[] = [0,0,0]\nfor (i in 0..3) { output[i] = i * 2 }");
    independent(&analysis);
    assert!(only_loop(&analysis).effects.may_fail);
}

/// Pure and impure lexical regions expose their compositional block decisions.
#[test]
fn block_purity_tracks_external_calls_and_captured_writes() {
    let (_, analysis) =
        source_analysis("let total = 0\n{ const first = 1 * 2 }\n{ print(total) }\n{ total = 2 }");
    assert_eq!(analysis.blocks.len(), 3);
    assert!(analysis.blocks[0].pure);
    assert!(!analysis.blocks[1].pure);
    assert!(!analysis.blocks[2].pure);
}

/// Returns its argument for effect-contract tests without touching external state.
fn identity(_: &ExecutionContext<'_>, arguments: &[Value]) -> Result<Option<Value>, CoreError> {
    Ok(arguments.first().cloned())
}

/// Discards its explicit argument under a trusted pure native contract.
fn discard(_: &ExecutionContext<'_>, _: &[Value]) -> Result<Option<Value>, CoreError> {
    Ok(None)
}
