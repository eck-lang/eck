use super::*;

/// Compiles a focused source fixture with the normal registry and computes its work table.
fn compiled_analysis(source: &str) -> (Registry, TypedProgram, ExecutionAnalysis) {
    let registry = crate::semantic::default_registry().unwrap();
    let syntax = crate::parse(source).unwrap();
    let program = crate::compile(&syntax, &registry).unwrap();
    let analysis = super::super::analyze(&program, &registry);
    (registry, program, analysis)
}

/// Estimates one top-level statement directly, isolating its compositional cost.
fn top_level_cost(registry: &Registry, program: &TypedProgram, index: usize) -> WorkCost {
    WorkEstimator {
        registry,
        analysis: &mut ExecutionAnalysis::default(),
    }
    .statement(&program.statements()[index])
}

/// Verifies exact costs for literal/local evaluation, declarations, assignments, and integer arithmetic.
#[test]
fn scalar_and_arithmetic_costs_follow_operation_classes() {
    let (registry, program, _) = compiled_analysis(
        "let x = 1\nx = x + 2\nlet multiplied = x * 2\nlet divided = x / 2\nlet remainder = x % 2",
    );
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 2);
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 8);
    assert_eq!(top_level_cost(&registry, &program, 2).units(), 13);
    assert_eq!(top_level_cost(&registry, &program, 3).units(), 33);
    assert_eq!(top_level_cost(&registry, &program, 4).units(), 33);
}

/// Checks literal evaluation, local reads, and plain assignment independently.
#[test]
fn literal_read_and_assignment_costs_are_exact() {
    let (registry, program, _) = compiled_analysis("let value = 1\n1\nvalue\nvalue = 2");
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 1);
    assert_eq!(top_level_cost(&registry, &program, 2).units(), 1);
    assert_eq!(top_level_cost(&registry, &program, 3).units(), 2);
}

/// Prices comparisons and expensive decimal arithmetic above their integer counterparts.
#[test]
fn comparisons_and_decimal_arithmetic_are_ordered_by_representation() {
    let (registry, program, _) = compiled_analysis(
        "let integer_comparison = 1 < 2\nlet decimal_comparison = 1.0 < 2.0\nlet integer_product = 2 * 3\nlet decimal_product = 2.0 * 3.0",
    );
    let costs = (0..program.statements().len())
        .map(|index| top_level_cost(&registry, &program, index).units())
        .collect::<Vec<_>>();
    assert_eq!(costs[0], 8);
    assert_eq!(costs[2], 13);
    assert!(costs[1] > costs[0]);
    assert!(costs[3] > costs[2]);
}

/// Counts array construction, indexed reads, and indexed writes with their operation costs.
#[test]
fn arrays_charge_for_creation_reads_and_writes() {
    let (registry, program, _) =
        compiled_analysis("let values: int[] = [1, 2]\nlet item = values[0]\nvalues[1] = 3");
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 13);
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 13);
    assert_eq!(top_level_cost(&registry, &program, 2).units(), 12);
}

/// Accounts for representation-only and scale-changing `->to` conversions.
#[test]
fn conversions_charge_for_casting_and_resolved_scaling() {
    let (registry, program, _) =
        compiled_analysis("let simple = 1 ->to(decimal)\nlet scaled = 1m ->to(cm)");
    let simple = top_level_cost(&registry, &program, 0).units();
    let scaled = top_level_cost(&registry, &program, 1).units();
    assert_eq!(simple, 12);
    assert!(scaled > simple);
}

/// Uses registered function metadata for both calls and pipeline syntax.
#[test]
fn calls_and_pipes_use_registered_function_costs() {
    let (registry, program, _) = compiled_analysis(
        "use String\nlet called = String.trim(\" value \" )\nlet piped = \" value \"->trim()",
    );
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 12);
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 12);
}

/// Expensive native work exceeds arithmetic and nested calls compose cached summaries.
#[test]
fn expensive_native_calls_and_nested_summaries_preserve_relative_order() {
    let (registry, program, _) = compiled_analysis(
        "use String\nlet arithmetic = 1 + 2\nlet expensive = String.replace(\"aba\", /a/g, \"X\")\nlet nested = String.uppercase(String.trim(\" x \"))",
    );
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 8);
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 104);
    assert_eq!(top_level_cost(&registry, &program, 2).units(), 42);
}

/// Finite dispatch adds its heaviest candidate rather than all exclusive executions.
#[test]
fn finite_arithmetic_dispatch_uses_maximum_candidate_work() {
    let (registry, program, _) =
        compiled_analysis("for (i in 0..2) { let first = i * 2\nlet second = first * 3 }");
    let TypedStatement::For { body, .. } = &program.statements()[0] else {
        panic!();
    };
    let TypedStatement::VariableDeclaration { expression, .. } = &body.statements[1] else {
        panic!();
    };
    let TypedExpressionKind::DynamicBinary {
        dispatch,
        left_operand,
        right_operand,
        ..
    } = &expression.kind
    else {
        panic!("widenable integer result must have finite dispatch");
    };
    let mut analysis = ExecutionAnalysis::default();
    let estimator = WorkEstimator {
        registry: &registry,
        analysis: &mut analysis,
    };
    let costs: Vec<_> = dispatch
        .plans
        .iter()
        .flatten()
        .map(|plan| estimator.binary_plan(plan.resolution.operator, &plan.execution_plan))
        .collect();
    assert!(costs.len() > 1);
    let children = estimator
        .expression(left_operand)
        .saturating_add(estimator.expression(right_operand));
    let expected = children.saturating_add(*costs.iter().max().unwrap());
    assert_eq!(estimator.expression(expression), expected);
    let incorrectly_summed = costs
        .iter()
        .fold(children, |cost, candidate| cost.saturating_add(*candidate));
    assert!(incorrectly_summed > expected);
}

/// Adds nested block work and chooses the heavier conditional branch after its condition.
#[test]
fn blocks_and_conditionals_compose_additively() {
    let (registry, program, _) =
        compiled_analysis("{ let first = 1\n { let second = 2 } }\nif (true) { let chosen = 3 }");
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 4);
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 3);
}

/// A conditional includes its condition and exactly the heavier complete branch.
#[test]
fn conditional_cost_is_condition_plus_maximum_branch() {
    let (registry, program, _) = compiled_analysis(
        "if (true) { let value = 2 * 3 } else { let value = 1 }\nif (false) { let value = 1 } else { let value = 2 * 3 }\n{ let value = 2 * 3 }\n{ let value = 1 }",
    );
    let heavy = top_level_cost(&registry, &program, 2);
    let cheap = top_level_cost(&registry, &program, 3);
    assert!(heavy > cheap);
    assert_eq!(
        top_level_cost(&registry, &program, 0),
        WorkCost::from_units(1).saturating_add(heavy)
    );
    assert_eq!(
        top_level_cost(&registry, &program, 1),
        WorkCost::from_units(1).saturating_add(heavy)
    );
    assert_ne!(
        top_level_cost(&registry, &program, 0),
        WorkCost::from_units(1)
            .saturating_add(heavy)
            .saturating_add(cheap)
    );
}

/// Scales known loop bodies by nonzero trip counts and charges no body for empty ranges.
#[test]
fn known_loop_counts_include_setup_and_handle_empty_ranges() {
    let (registry, program, analysis) = compiled_analysis(
        "for (i in 0..3) { let value = 1 }\nfor (j in 4..4) { let value = 1 }\nfor (k in 5..2) { let value = 1 }",
    );
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 28);
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 7);
    assert_eq!(top_level_cost(&registry, &program, 2).units(), 7);
    let mut iteration_costs = analysis
        .loops
        .values()
        .map(|plan| plan.cost_per_iteration.units())
        .collect::<Vec<_>>();
    iteration_costs.sort_unstable();
    assert_eq!(iteration_costs, [7, 7, 7]);
}

/// Includes nested known-loop work once in its parent and stores each loop's own iteration cost.
#[test]
fn nested_known_loops_are_compositional_and_attached_once() {
    let (registry, program, analysis) =
        compiled_analysis("for (outer in 0..2) { for (inner in 0..3) { let value = 1 } }");
    assert_eq!(top_level_cost(&registry, &program, 0).units(), 73);
    let mut iteration_costs = analysis
        .loops
        .values()
        .map(|plan| plan.cost_per_iteration.units())
        .collect::<Vec<_>>();
    iteration_costs.sort_unstable();
    assert_eq!(iteration_costs, [7, 33]);
}

/// Applies one conservative body fallback and very-high overhead at each unknown loop level.
#[test]
fn nested_unknown_loop_bounds_fall_back_once_per_loop() {
    let (registry, program, analysis) = compiled_analysis(
        "let limit = 4\nfor (outer in 0..limit) { for (inner in 0..limit) { let value = 1 } }",
    );
    assert_eq!(top_level_cost(&registry, &program, 1).units(), 226);
    let mut iteration_costs = analysis
        .loops
        .values()
        .map(|plan| plan.cost_per_iteration.units())
        .collect::<Vec<_>>();
    iteration_costs.sort_unstable();
    assert_eq!(iteration_costs, [7, 119]);
}

/// Saturates direct composition and estimates an extreme signed range without wrapping.
#[test]
fn work_arithmetic_and_extreme_trip_counts_saturate() {
    let maximum = WorkCost::from_units(u64::MAX);
    assert_eq!(
        maximum.saturating_add(WorkCost::from_units(1)).units(),
        u64::MAX
    );
    assert_eq!(
        WorkCost::from_units(2).saturating_mul(u64::MAX).units(),
        u64::MAX
    );

    let (registry, program, _) =
        compiled_analysis("for (i in -9223372036854775808..9223372036854775807) { let value = 1 }");
    assert_eq!(top_level_cost(&registry, &program, 0).units(), u64::MAX);
}
