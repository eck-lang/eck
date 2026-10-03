use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::thread::ThreadId;

use super::*;
use crate::ir::TypedProgram;
use crate::semantic::{
    CoreError, ExecutionContext, FunctionEffectSummary, FunctionSignature, Registry,
};

/// Builds a default registry with deterministic arithmetic and failing test natives.
fn test_registry() -> Registry {
    let mut registry = crate::default_registry().unwrap();
    let integer = registry.default_integer().unwrap();
    registry
        .register_global_function_with_effect_summary(
            "twice",
            FunctionSignature::Exact(vec![integer]),
            Some(integer),
            twice,
            FunctionEffectSummary::PURE,
        )
        .unwrap();
    registry
        .register_global_function_with_effect_summary(
            "checked",
            FunctionSignature::Exact(vec![integer]),
            Some(integer),
            checked,
            FunctionEffectSummary {
                may_fail: true,
                ..FunctionEffectSummary::PURE
            },
        )
        .unwrap();
    registry
}

/// Returns deterministic CPU arithmetic with no ECK-visible instrumentation.
fn twice(context: &ExecutionContext<'_>, arguments: &[Value]) -> Result<Option<Value>, CoreError> {
    let input = *arguments[0].downcast_ref::<i64>().unwrap();
    Ok(Some(Value::new(
        context.registry().default_integer()?,
        input * 2,
    )))
}

/// Fails at multiple logical indices so scheduling cannot choose the reported error.
fn checked(
    _context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let input = *arguments[0].downcast_ref::<i64>().unwrap();
    if input == 3 || input == 19 {
        return Err(CoreError::Runtime(format!("failure at {input}")));
    }
    Ok(Some(arguments[0].clone()))
}

/// Resolves source using the ordinary parser/compiler pipeline, including analysis.
fn compile_source(source: &str, registry: &Registry) -> TypedProgram {
    crate::compile(&crate::parse(source).unwrap(), registry).unwrap()
}

/// Supplies a known budget independent of machine topology or timing.
fn executor(workers: usize) -> Executor {
    Executor::new(ExecutionOptions {
        workers,
        minimum_parallel_iterations: 0,
    })
    .unwrap()
}

/// Executes with a test-only worker observer and returns the exact surviving state.
fn run_collecting(
    program: &TypedProgram,
    registry: &Registry,
    executor: &Executor,
    observer: Option<&(dyn Fn(i64, ThreadId) + Sync)>,
) -> (Result<(), RuntimeError>, Vec<Option<Value>>) {
    let session = ExecutionSession {
        analysis: &program.execution_analysis,
        identity: program.execution_identity,
        executor,
        observer,
    };
    let mut runtime = Runtime {
        registry,
        configuration: registry.default_runtime_configuration(),
        local_values: vec![None; program.local_slot_count],
        loop_control: None,
        parallel_execution: Some(&session),
        pending_writes: None,
    };
    let result = program
        .statements
        .iter()
        .try_for_each(|statement| runtime.execute_statement(statement));
    (result, runtime.local_values)
}

/// Reads one named surviving array as plain integer values for exact comparisons.
fn array_contents(program: &TypedProgram, locals: &[Option<Value>], name: &str) -> Vec<i64> {
    let slot = program
        .bindings
        .iter()
        .find(|binding| binding.name == name)
        .unwrap()
        .slot;
    crate::ArrayValue::from_value(locals[slot.0].as_ref().unwrap())
        .unwrap()
        .elements()
        .iter()
        .map(|element| *element.downcast_ref::<i64>().unwrap())
        .collect()
}

/// Creates writable output and immutable input using documented array literals.
fn array_source(length: usize, body: &str) -> String {
    let output = vec!["0"; length].join(",");
    let input = (0..length)
        .map(|index| index.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "let output: int[] = [{output}]\nconst input: int[] = [{input}]\nfor (i in 0..{length}) {{ {body} }}"
    )
}

/// Proves every iteration ran exactly once on multiple actual worker identities.
#[test]
fn real_workers_execute_disjoint_writes_exactly_once() {
    let registry = test_registry();
    let program = compile_source(
        &array_source(257, "output[i] = twice(twice(input[i]))"),
        &registry,
    );
    let executor = executor(4);
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    let (result, locals) = run_collecting(&program, &registry, &executor, Some(&observe));
    result.unwrap();
    assert_eq!(
        array_contents(&program, &locals, "output"),
        (0..257).map(|index| index * 4).collect::<Vec<_>>()
    );
    let observations = observations.into_inner().unwrap();
    let mut counts = HashMap::new();
    let mut threads = HashSet::new();
    for (iteration, thread) in observations {
        *counts.entry(iteration).or_insert(0) += 1;
        threads.insert(thread);
    }
    assert_eq!(counts.len(), 257);
    assert!(counts.values().all(|count| *count == 1));
    assert!(threads.len() >= 2);
    assert!(!threads.contains(&std::thread::current().id()));
}

/// Rejects loop-carried scalar state without ever entering the worker path.
#[test]
fn unsafe_scalar_loop_keeps_sequential_dispatch() {
    let registry = test_registry();
    let program = compile_source(
        "let total: int = 0\nfor (i in 0..100) { total = total + i }",
        &registry,
    );
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    let (result, locals) = run_collecting(&program, &registry, &executor(4), Some(&observe));
    result.unwrap();
    assert!(observations.into_inner().unwrap().is_empty());
    assert_eq!(
        locals[0].as_ref().unwrap().downcast_ref::<i64>(),
        Some(&4950)
    );
}

/// Compares arithmetic, branch scopes, arrays and pure call chains exactly.
#[test]
fn branch_blocks_and_continue_match_sequential_execution() {
    let registry = test_registry();
    let program = compile_source(
        &array_source(
            129,
            "if (i % 3 == 0) { continue }\nlet first = input[i] * 2\nif (i % 2 == 0) { let second = twice(first)\noutput[i] = second + 1 } else { let second = first + 10\noutput[i] = second * second }",
        ),
        &registry,
    );
    let sequential = run_collecting(&program, &registry, &executor(1), None);
    let parallel = run_collecting(&program, &registry, &executor(4), None);
    sequential.0.unwrap();
    parallel.0.unwrap();
    assert_eq!(
        array_contents(&program, &sequential.1, "output"),
        array_contents(&program, &parallel.1, "output")
    );
}

/// Makes earlier writes in one iteration visible to its following reads.
#[test]
fn per_iteration_write_overlay_matches_sequential_reads() {
    let registry = test_registry();
    let program = compile_source(
        &array_source(65, "output[i] = i\noutput[i] = output[i] + 1"),
        &registry,
    );
    let (result, locals) = run_collecting(&program, &registry, &executor(4), None);
    result.unwrap();
    assert_eq!(
        array_contents(&program, &locals, "output"),
        (1..=65).collect::<Vec<_>>()
    );
}

/// Preserves value-copy aliases while replacing the independent destination.
#[test]
fn copy_on_write_alias_keeps_original_snapshot() {
    let registry = test_registry();
    let source = format!(
        "let input: int[] = [{}]\nlet output = input\nfor (i in 0..67) {{ output[i] = input[i] * 2 }}",
        (0..67)
            .map(|index| index.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let program = compile_source(&source, &registry);
    let (result, locals) = run_collecting(&program, &registry, &executor(4), None);
    result.unwrap();
    assert_eq!(
        array_contents(&program, &locals, "input"),
        (0..67).collect::<Vec<_>>()
    );
    assert_eq!(
        array_contents(&program, &locals, "output"),
        (0..67).map(|index| index * 2).collect::<Vec<_>>()
    );
}

/// Selects the lowest failing iteration and commits precisely its completed prefix.
#[test]
fn errors_preserve_first_failure_and_partial_iteration_writes() {
    let registry = test_registry();
    let program = compile_source(
        &array_source(
            32,
            "output[i] = i + 10\nlet result = checked(i)\noutput[i] = result + 100",
        ),
        &registry,
    );
    for _ in 0..20 {
        let sequential = run_collecting(&program, &registry, &executor(1), None);
        let parallel = run_collecting(&program, &registry, &executor(4), None);
        assert_eq!(
            sequential.0.unwrap_err().to_string(),
            parallel.0.unwrap_err().to_string()
        );
        assert_eq!(
            array_contents(&program, &sequential.1, "output"),
            array_contents(&program, &parallel.1, "output")
        );
        assert_eq!(array_contents(&program, &parallel.1, "output")[3], 13);
        assert_eq!(array_contents(&program, &parallel.1, "output")[4], 0);
    }
}

/// Ensures an invalid store keeps earlier stores and leaves its destination untouched.
#[test]
fn out_of_bounds_write_matches_sequential_error_prefix() {
    let registry = test_registry();
    let program = compile_source(
        "let output: int[] = [0,0,0,0]\nfor (i in 0..12) { output[i] = i + 1 }",
        &registry,
    );
    let sequential = run_collecting(&program, &registry, &executor(1), None);
    let parallel = run_collecting(&program, &registry, &executor(4), None);
    assert_eq!(
        sequential.0.unwrap_err().to_string(),
        parallel.0.unwrap_err().to_string()
    );
    assert_eq!(
        array_contents(&program, &sequential.1, "output"),
        array_contents(&program, &parallel.1, "output")
    );
}

/// Exercises repeated invocations, complete batches and tails on one reusable pool.
#[test]
fn repeated_execution_reuses_workers_without_lost_or_corrupt_elements() {
    let registry = test_registry();
    let program = compile_source(
        &array_source(4133, "output[i] = input[i] * 2 + 1"),
        &registry,
    );
    let executor = executor(4);
    let worker_identities = Mutex::new(HashSet::new());
    let observe = |_iteration, thread| {
        worker_identities.lock().unwrap().insert(thread);
    };
    for _ in 0..12 {
        let (result, locals) = run_collecting(&program, &registry, &executor, Some(&observe));
        result.unwrap();
        assert_eq!(
            array_contents(&program, &locals, "output"),
            (0..4133).map(|index| index * 2 + 1).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        worker_identities.into_inner().unwrap().len(),
        4,
        "repeated invocations must reuse the same bounded workers"
    );
}

/// Executes only the inner eligible loop level, preventing nested pool multiplication.
#[test]
fn nested_loops_use_one_parallel_level() {
    let registry = test_registry();
    let program = compile_source(
        "for (outer in 0..3) { for (i in 0..17) { let result = twice(i + outer) } }",
        &registry,
    );
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    let (result, _) = run_collecting(&program, &registry, &executor(4), Some(&observe));
    result.unwrap();
    let observations = observations.into_inner().unwrap();
    assert_eq!(observations.len(), 51);
    assert!(
        observations
            .iter()
            .map(|(_, thread)| *thread)
            .collect::<HashSet<_>>()
            .len()
            <= 4
    );
}

/// Honors the profitability threshold even when static independence is proven.
#[test]
fn tiny_ranges_do_not_start_workers() {
    let registry = test_registry();
    let program = compile_source("for (i in 0..2) { let value = twice(i) }", &registry);
    let executor = Executor::new(ExecutionOptions {
        workers: 4,
        minimum_parallel_iterations: 100,
    })
    .unwrap();
    let (result, _) = run_collecting(&program, &registry, &executor, None);
    result.unwrap();
    assert!(executor.pool.lock().unwrap().is_none());
}

/// Keeps empty, reversed and negative ranges equivalent under forced scheduling.
#[test]
fn range_edges_match_sequential_execution() {
    let registry = test_registry();
    for source in [
        "for (i in 0..0) { let value = twice(i) }",
        "for (i in 10..2) { let value = twice(i) }",
        "for (i in -19..12) { let value = twice(i) }",
    ] {
        let program = compile_source(source, &registry);
        run_collecting(&program, &registry, &executor(1), None)
            .0
            .unwrap();
        run_collecting(&program, &registry, &executor(4), None)
            .0
            .unwrap();
    }
}

/// Rejects an invalid worker budget before any program executes.
#[test]
fn zero_workers_are_rejected() {
    assert!(
        Executor::new(ExecutionOptions {
            workers: 0,
            minimum_parallel_iterations: 0
        })
        .is_err()
    );
}

/// Produces an unchanged result through an appended extension configuration hook.
fn appended_integer_transform(
    value: &Value,
    _configuration: &crate::semantic::RuntimeConfiguration,
) -> Result<Value, CoreError> {
    Ok(value.clone())
}

/// Invalidates a compiled safety proof after existing type behavior acquires a callback.
#[test]
fn registry_callback_changes_disable_previously_proven_parallelism() {
    let mut registry = test_registry();
    let program = compile_source("for (i in 0..31) { let value = i * 2 }", &registry);
    let integer = registry.default_integer().unwrap();
    registry
        .register_type_configuration(
            integer,
            crate::semantic::TypeConfigurationDescriptor {
                transform_result: Some(appended_integer_transform),
                transform_owned_result: None,
                initial_result_transform_is_identity: true,
                format: None,
            },
        )
        .unwrap();
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    let (result, _) = run_collecting(&program, &registry, &executor(4), Some(&observe));
    result.unwrap();
    assert!(observations.into_inner().unwrap().is_empty());
}

/// Keeps colliding element stores and undeclared/effectful calls on the caller thread.
#[test]
fn unsafe_array_indices_and_native_effects_never_dispatch_workers() {
    let mut registry = test_registry();
    let integer = registry.default_integer().unwrap();
    registry
        .register_global_function(
            "unknown",
            FunctionSignature::Exact(vec![integer]),
            Some(integer),
            twice,
        )
        .unwrap();
    registry
        .register_global_function_with_effect_summary(
            "effectful",
            FunctionSignature::Exact(vec![integer]),
            Some(integer),
            twice,
            FunctionEffectSummary {
                purity: crate::semantic::FunctionPurity::Impure,
                external_effect: crate::semantic::FunctionExternalEffect::WritesExternalState,
                ..FunctionEffectSummary::PURE
            },
        )
        .unwrap();
    for body in [
        "output[0] = i",
        "output[i % 4] = i",
        "output[i] = unknown(i)",
        "output[i] = effectful(i)",
    ] {
        let program = compile_source(&array_source(32, body), &registry);
        let observations = Mutex::new(Vec::new());
        let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
        let sequential = run_collecting(&program, &registry, &executor(1), None);
        let parallel = run_collecting(&program, &registry, &executor(4), Some(&observe));
        sequential.0.unwrap();
        parallel.0.unwrap();
        assert_eq!(
            array_contents(&program, &sequential.1, "output"),
            array_contents(&program, &parallel.1, "output")
        );
        assert!(observations.into_inner().unwrap().is_empty());
    }
}

/// Exercises injective affine indices with offsets, strides and negative coefficients.
#[test]
fn affine_stride_and_reverse_writes_match_sequential_results() {
    let registry = test_registry();
    for index_expression in ["2 * i + 1", "i * 2 + 1", "63 - i", "i + 5", "i - 0"] {
        let source = format!(
            "let output: int[] = [{}]\nfor (i in 0..32) {{ output[{index_expression}] = twice(i) }}",
            vec!["0"; 64].join(",")
        );
        let program = compile_source(&source, &registry);
        let observations = Mutex::new(Vec::new());
        let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
        let sequential = run_collecting(&program, &registry, &executor(1), None);
        let parallel = run_collecting(&program, &registry, &executor(4), Some(&observe));
        sequential.0.unwrap();
        parallel.0.unwrap();
        assert_eq!(
            array_contents(&program, &sequential.1, "output"),
            array_contents(&program, &parallel.1, "output")
        );
        assert_eq!(
            observations.into_inner().unwrap().len(),
            32,
            "affine index {index_expression} did not run on workers"
        );
    }
}

/// Invalidates cached independence before exposing a compiled tree for rewriting.
#[test]
fn editing_resolved_statements_cannot_reuse_a_stale_parallel_proof() {
    let registry = test_registry();
    let mut program = compile_source(
        "let total: int = 0\nfor (i in 0..32) { let result = i * 2 }",
        &registry,
    );
    let unsafe_program = compile_source(
        "let total: int = 0\nfor (i in 0..32) { total = total + i }",
        &registry,
    );
    assert!(!program.execution_analysis().loops.is_empty());
    program.statements_mut()[1] = unsafe_program.statements()[1].clone();
    assert!(program.execution_analysis().loops.is_empty());
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    let (result, locals) = run_collecting(&program, &registry, &executor(4), Some(&observe));
    result.unwrap();
    assert_eq!(
        locals[0].as_ref().unwrap().downcast_ref::<i64>(),
        Some(&496)
    );
    assert!(observations.into_inner().unwrap().is_empty());
}

/// Represents an extension callback registered after a program was compiled.
fn later_binary_operator(left: &Value, _right: &Value) -> Result<Value, CoreError> {
    Ok(left.clone())
}

/// Invalidates the callback inventory when a previously absent operation is installed.
#[test]
fn extending_registry_dispatch_cannot_keep_a_stale_callback_inventory() {
    let mut registry = test_registry();
    let mut descriptor = registry
        .type_descriptor(registry.default_integer().unwrap())
        .unwrap()
        .clone();
    descriptor.id = registry.allocate_type_id();
    descriptor.name = "later_integer";
    let later_integer = descriptor.id;
    registry.register_type(descriptor).unwrap();
    let program = compile_source("for (i in 0..31) { let value = i * 2 }", &registry);
    assert!(
        program
            .execution_analysis()
            .loops
            .values()
            .any(|entry| matches!(
                entry.parallelism,
                crate::analysis::Parallelism::IndependentIterations { .. }
            ))
    );
    registry
        .register_binary_operator(
            crate::semantic::BinaryOperator::Addition,
            later_integer,
            later_integer,
            later_integer,
            later_binary_operator,
        )
        .unwrap();
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    run_collecting(&program, &registry, &executor(4), Some(&observe))
        .0
        .unwrap();
    assert!(observations.into_inner().unwrap().is_empty());
}

/// Source budgets at or below one never start workers and preserve array results.
#[test]
fn source_cores_at_or_below_one_disable_parallelism() {
    let registry = test_registry();
    for cores in [-7, 0, 1] {
        let program = compile_source(
            &format!(
                "@config {{ \"cores\": {cores} }}\n{}",
                array_source(31, "output[i] = input[i] * 2")
            ),
            &registry,
        );
        let executor = executor(4);
        let observations = Mutex::new(Vec::new());
        let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
        let (result, locals) = run_collecting(&program, &registry, &executor, Some(&observe));
        result.unwrap();
        assert!(observations.into_inner().unwrap().is_empty());
        assert!(executor.pool.lock().unwrap().is_none());
        assert_eq!(
            array_contents(&program, &locals, "output"),
            (0..31).map(|index| index * 2).collect::<Vec<_>>()
        );
    }
}

/// Explicit source configuration supplies four workers even with a serial default.
#[test]
fn source_cores_four_overrides_executor_default() {
    let registry = test_registry();
    let program = compile_source(
        "@config { \"cores\": 4 }\nfor (i in 0..257) { let value = i * 2 }",
        &registry,
    );
    let executor = executor(1);
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    run_collecting(&program, &registry, &executor, Some(&observe))
        .0
        .unwrap();
    let observations = observations.into_inner().unwrap();
    assert_eq!(observations.len(), 257);
    assert_eq!(
        observations
            .iter()
            .map(|(_, thread)| thread)
            .collect::<HashSet<_>>()
            .len(),
        4
    );
}

/// Root overrides apply to later loops and serial sections retain the parallel pool.
#[test]
fn source_cores_switches_dispatch_and_reuses_pool() {
    let registry = test_registry();
    let source = "@config { cores: 1 }\nfor (i in 0..31) { let x = i * 2 }\n@config { cores: 4 }\nfor (i in 100..131) { let x = i * 2 }\n@config { cores: 0 }\nfor (i in 200..231) { let x = i * 2 }\n@config { cores: 4 }\nfor (i in 300..331) { let x = i * 2 }";
    let program = compile_source(source, &registry);
    let executor = executor(1);
    let observations = Mutex::new(Vec::new());
    let observe = |iteration, thread| observations.lock().unwrap().push((iteration, thread));
    run_collecting(&program, &registry, &executor, Some(&observe))
        .0
        .unwrap();
    let first_pool = executor.worker_pool(4).unwrap();
    let observations = observations.into_inner().unwrap();
    assert_eq!(observations.len(), 62);
    assert!(
        observations
            .iter()
            .all(|(index, _)| (100..131).contains(index) || (300..331).contains(index))
    );
    let first_threads = observations
        .iter()
        .filter(|(index, _)| *index < 200)
        .map(|(_, thread)| thread)
        .collect::<HashSet<_>>();
    let second_threads = observations
        .iter()
        .filter(|(index, _)| *index > 200)
        .map(|(_, thread)| thread)
        .collect::<HashSet<_>>();
    assert_eq!(first_threads, second_threads);
    run_collecting(&program, &registry, &executor, None)
        .0
        .unwrap();
    assert!(Arc::ptr_eq(&first_pool, &executor.worker_pool(4).unwrap()));
    let second_pool = executor.worker_pool(2).unwrap();
    assert_eq!(second_pool.current_num_threads(), 2);
    assert!(!Arc::ptr_eq(&first_pool, &second_pool));
}

/// Numeric overrides keep conservative sequential dispatch despite a cores override.
#[test]
fn source_cores_does_not_ignore_numeric_configuration() {
    let registry = test_registry();
    let program = compile_source(
        "@config { cores: 4\ndecimal: { scale: 2 } }\nfor (i in 0..31) { let x = i * 2 }",
        &registry,
    );
    let executor = executor(4);
    let observe = |_, _| panic!("numeric configuration must retain sequential execution");
    run_collecting(&program, &registry, &executor, Some(&observe))
        .0
        .unwrap();
    assert!(executor.pool.lock().unwrap().is_none());
}

/// Rejects enum-like, optional and fractional worker budgets during compilation.
#[test]
fn source_cores_rejects_noninteger_values() {
    let registry = test_registry();
    for value in ["HalfEven", "None", "1.5"] {
        let program = crate::parse(&format!("@config {{ cores: {value} }}")).unwrap();
        assert!(crate::compile(&program, &registry).is_err());
    }
}

/// An extension can observe scheduling configuration in its result transformer.
fn cores_integer_transform(
    value: &Value,
    configuration: &crate::semantic::RuntimeConfiguration,
) -> Result<Value, CoreError> {
    if configuration.uses_initial_values() {
        return Ok(value.clone());
    }
    let adjustment = match configuration.value("cores") {
        Some(crate::semantic::ConfigurationValue::Integer(cores)) => *cores,
        _ => 0,
    };
    Ok(Value::new(
        value.type_id(),
        value.downcast_ref::<i64>().unwrap() + adjustment,
    ))
}

/// Scheduling overrides must not skip extension result hooks that read cores.
#[test]
fn source_cores_preserves_extension_result_transform() {
    let mut registry = test_registry();
    let integer = registry.default_integer().unwrap();
    registry
        .register_type_configuration(
            integer,
            crate::semantic::TypeConfigurationDescriptor {
                transform_result: Some(cores_integer_transform),
                transform_owned_result: None,
                initial_result_transform_is_identity: true,
                format: None,
            },
        )
        .unwrap();
    let program = compile_source(
        "@config { cores: 4 }\nlet output: int[] = [0]\nfor (i in 0..1) { let value = 2 * 3\noutput[0] = value }",
        &registry,
    );
    let (result, locals) = run_collecting(&program, &registry, &executor(4), None);
    result.unwrap();
    assert_eq!(array_contents(&program, &locals, "output"), vec![10]);
}

/// Applies an observable adjustment to a promoted integer under source configuration.
fn cores_wide_integer_transform(
    value: &Value,
    configuration: &crate::semantic::RuntimeConfiguration,
) -> Result<Value, CoreError> {
    if configuration.uses_initial_values() {
        return Ok(value.clone());
    }
    Ok(Value::new(
        value.type_id(),
        value.downcast_ref::<i128>().unwrap() + 4,
    ))
}

/// Direct binary plans must transform the actual promoted result type.
#[test]
fn source_cores_preserves_promoted_result_transform() {
    let mut registry = test_registry();
    let wide_integer = registry.type_by_name("int128").unwrap();
    registry
        .register_type_configuration(
            wide_integer,
            crate::semantic::TypeConfigurationDescriptor {
                transform_result: Some(cores_wide_integer_transform),
                transform_owned_result: None,
                initial_result_transform_is_identity: false,
                format: None,
            },
        )
        .unwrap();
    let program = compile_source(
        "@config { cores: 1 }\nlet output = 0\nfor (i in 0..1) { let promoted = 9223372036854775807 * 2\noutput = promoted }",
        &registry,
    );
    let (result, locals) = run_collecting(&program, &registry, &executor(4), None);
    result.unwrap();
    let output = program
        .bindings
        .iter()
        .find(|binding| binding.name == "output")
        .unwrap()
        .slot;
    assert_eq!(
        locals[output.0].as_ref().unwrap().downcast_ref::<i128>(),
        Some(&18446744073709551618)
    );
}
