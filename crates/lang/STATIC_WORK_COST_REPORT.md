# Static work-cost profitability report

Completed on 3 October 2026 in the current working tree. Implementation stays
inside `crates/lang`; the existing parallel-safety analysis and executor are
extended rather than replaced. Existing unrelated working-tree changes remain.

## Source configuration follow-up

The current user-facing configuration is now:

```eck
@config {
    parallelization: {
        cores: 4
        level: 50
    }
}
```

Higher levels admit progressively smaller safe workloads. Level 0 disables
parallel execution, level 50 maps to the existing 750,000-unit default, and
level 100 admits every nonempty safe supported range when runtime guards pass.
Levels are integers from 0 through 100, not probabilities. Intermediate
threshold anchors halve every ten levels, with integer linear interpolation.
The compiler caches this conversion once. The raw Rust executor threshold
remains available for controlled testing and benchmarking. The old root
`cores` and source `parallelization.threshold` paths are no longer registered.

Follow-up validation: `cargo test-all` passes 951 language-library tests, five
CLI integration tests, 44 runner tests, one doctest and 556 language cases.
The parallel runtime tests pass 33 cases, including monotonic level selection,
exact array results, disabled saturated work, maximum-level worker guards,
later configuration updates and extension callback visibility. Configuration
tests cover all 101 accepted levels and reject both removed source paths.

The complete release benchmark also passes with the nested source configuration:
level 50 retains the previous matrix decisions. Source-selected one/four/ten
workers measure 1515.535417 / 398.505209 / 253.372500 ms at one million iterations
and 5969.784750 / 1579.396209 / 997.642417 ms at four million iterations. New
`level-default-work-results.csv` and `level-source-cores-results.csv` preserve
this rerun separately from historical measurements. Unpaired timing changes
are not evidence that this configuration change improves execution speed.

The current core-count default is `null`, which automatically selects 60% of
available logical CPUs, rounded down with at least one worker. Null has its own
syntax and semantic representation; cores no longer accepts the legacy None
option. Zero and one remain sequential, and larger nonnegative integers select
an explicit budget, including 100,000. Level 50 remains the conservative default.
A later null override restores the automatic budget while preserving the level.
This supersedes the initial null-as-disabled contract in commit 4789cdd. The
previous validation and timing records below refer to that earlier contract.

Automatic-budget correction validation on 4 October 2026: `cargo test-all`
passes 957 language-library tests, five CLI integration tests, 45 runner tests,
one doctest and 561 language cases. All 13 automatic-parallelization language
cases pass, including omitted/null cores and rejection of `None`. Rust coverage
checks rounding, overflow, restoring automatic budgets and actual worker
dispatch. The release source-core compile-only benchmark accepts all six
one/four/ten-worker variants. Formatting and diff checks pass. No timing
measurements were rerun for this correction.

Core-count validation on 4 October 2026: the complete working-tree suite passes
953 language-library tests, five CLI integration tests, 44 runner tests, one
doctest and 559 language cases. All 11 automatic-parallelization language cases
pass, including null/None, 100,000 cores with level zero, and negative-core
rejection. The release benchmark compile-only check accepts all one/four/ten
core source variants at both one and four million iterations. Existing timed
measurements above remain historical and were not rerun for this cold-path
configuration decoding change.

An isolated export of the staged commit also passes the locked, offline Rust
workspace suite and all 11 feature language cases. It includes only the missing
CSV dependency declarations needed by already committed source, the benchmark
target, and the parallelization documentation index; unrelated manifest version
updates, CSV/runner edits and other pending features are excluded.

The numbered report below records the original static-work implementation and
its measured evidence. Its source syntax and threshold fixture names are
historical; see the authoritative automatic-parallelization document for the
current contract. The original measurements remain unchanged.

## 1. Architecture found

`analysis/resolved.rs` consumes typed IR, resolves effects and dependencies by
binding/slot identity, and classifies loops as sequential or independent.
`analysis/resolved/builtins.rs` verifies registered callback identities.
`analysis/effects.rs` defines the retained evidence and loop plans.
`TypedProgram.execution_analysis()` owns the immutable source-span side table;
IR mutation invalidates it. Native effects live in `FunctionEffectSummary`.

`runtime/runtime/parallel_execution.rs` owns the reusable Rayon pool, runtime
guards, worker snapshots, bounded batches and ordered array-write journals.
Configuration already had the source `cores` setting. The previous profitability
test required 50,000 iterations. Existing thread observers, semantic/failure
tests and the in-process release benchmark were reused.

## 2. Files changed by this task

Implementation and Rust tests:

- `src/analysis/work.rs` and `work.tests.rs` — new visitor, central classes and tests.
- `src/analysis/mod.rs`, `effects.rs`, `resolved.rs` — attach static costs to existing analysis.
- `src/runtime/runtime/parallel_execution.rs` and `parallel_execution.tests.rs` — consume cost and test dispatch.
- `src/semantic/configuration.rs` and `configuration.tests.rs` — decoded source threshold and validation tests.
- `src/semantic/registry/bootstrap.rs` — register the existing-schema configuration leaf.
- `src/semantic/descriptor.rs`, `registry/functions.rs`, `registry/functions.tests.rs` — independent native cost metadata.
- `src/primitives/string/functions/mod.rs` — pure built-in cost audit.
- `benches/automatic_parallelization.rs` — controlled work/size matrix.

Paths above are relative to `crates/lang`. Other artifacts:

- `.agents/docs/eck-lang/automatic-parallelization.md` — authoritative behavior.
- `AGENTS.md` — synchronize that document's index description.
- `testing/use-cases/language/automatic-parallelization/threshold_{zero,high,negative_rejected,fractional_rejected}.eckt` — four new CLI cases.
- `testing/benchmarks/language/automatic-parallelization/workload-cheap.eck` — new cheap fixture.
- That benchmark directory's `README.md`, `work-cost-results.csv`, `work-cost-source-cores-results.csv`, and `work-cost-baseline-results.csv` — methodology and measured evidence.
- `TODO/Desired/adaptive-parallel-cost-learning.md` — future-only design.
- `crates/lang/STATIC_WORK_COST_REPORT.md` — this report.

No crate, dependency, parser production, language function syntax or worker
execution architecture was introduced. Earlier reports and measurements remain
historical evidence; this report documents the new work threshold.

## 3. Representation

`WorkCost` wraps nonnegative `u64` relative units. It exposes exact construction,
unit inspection, saturating addition and saturating trip-count multiplication.
Empty blocks cost zero. Range bookkeeping makes even an empty iteration cost
five units. Costs cannot be negative or NaN, and overflow becomes `u64::MAX`.
The stored scheduling output is one `cost_per_iteration`, not category scores.

## 4. Central table

| Class | Units |
| --- | ---: |
| Trivial | 1 |
| Low | 5 |
| Medium | 10 |
| High | 30 |
| VeryHigh | 100 |

The class mapping, source configuration path and default threshold are central
constants in `analysis/work.rs`. Units are neither nanoseconds nor normalized
magnitude.

## 5. Primitive operations

Literals, local reads and binding/assignment bookkeeping are Trivial. Boolean
operations, ordinary comparisons and addition/subtraction are Low. Array
access/store/end operations, multiplication and base casts are Medium.
Division/remainder, decimal/bigint arithmetic/comparisons and nonidentity
conversions are High. Power is VeryHigh.

The visitor reads resolved operator descriptors and operand type identities;
it adds compiled scale steps and relative adjustments. Each scale factor costs
Trivial plus its resolved operator. Finite dispatch takes the maximum candidate
plan, avoiding a sum of mutually exclusive alternatives. Identity conversion
is Trivial; nonidentity scaling also includes its compiled steps. Unsupported
operations receive a deterministic fallback without gaining eligibility.

## 6. Native functions

`FunctionDescriptor.work_cost` is independent of effects. Existing registration
APIs default to High, including pure functions lacking a specialized estimate.
`Registry::set_function_work_cost` validates the function ID and updates the
dense descriptor. The exact cached scalar can also hold a future transitive
function-body summary; no user-defined function-body IR exists today.

The pure built-ins are String transformations: trim helpers are Medium; case
transforms, padding, normalization and literal replace/remove are High; repeat
and regex replacement are VeryHigh. Purity declarations are unchanged. Unknown
purity remains unsafe regardless of cost. Costs are read during compilation,
not at every runtime call; metadata changes affect subsequently compiled plans.

## 7. Composition

Sequential statements and lexical blocks add complete child costs, without a
fixed block surcharge. Declarations/assignments add Trivial to their expression;
indexed stores add Medium to the index and value computations. Calls add
argument costs and one cached callee summary; pipes also add their base value.
Short-circuit boolean expressions conservatively include both operands.

## 8. Conditionals

`cost(if) = cost(condition) + max(cost(then), cost(else))`.
Missing `else` costs zero. Tests cover heavier branches on either side and
prove the estimator does not add both alternatives. There is no branch
prediction or probability model.

## 9. Known nested loops

Immediate signed literal bounds determine `N` without executing callbacks.
A nested range contributes bounds evaluation, five setup units, and
`N * (body_cost + 5)`. This composes recursively. Empty/reversed ranges add
no body work. Every nested loop receives its own iteration summary, while the
containing safety decision remains unchanged and sequential today.

## 10. Unknown nested loops

Unknown range bounds contribute one body iteration including bookkeeping plus
a VeryHigh surcharge, as well as bounds/setup. Unknown while/iterable regions
contribute one condition/source and body plus that surcharge. The model is
bounded and deterministic, not a symbolic engine or mathematical upper bound.

## 11. Configuration

```eck
@config {
    cores: 4
    parallelization: {
        threshold: 750000
    }
}
```

The schema leaf is `parallelization.threshold`. It accepts integers from zero
through `i64::MAX`, rejecting negative/fractional values, symbols including
NaN/Infinity, `None`, and oversized numeric representations before execution.
Source overrides take precedence over executor defaults, persist until changed,
and are decoded once into typed execution state. Both scheduling leaves retain
built-in identity fast paths while remaining observable to extension callbacks.

The executor field is now `ExecutionOptions.parallelization_threshold: u64`,
replacing `minimum_parallel_iterations`. Existing one-worker/source `cores: 1`
sequential control is preserved. Zero forces nonempty safe ranges, including
singletons, through the worker path when all existing guards pass.

## 12. Default and calibration

The default is **750,000 work units**. The initial three-shape experiment found
that 10,000 indexed writes (590,000 units) lost to sequential execution, while
1,000 heavy arithmetic iterations (753,000 units) won substantially. The chosen
threshold avoids that measured replay loss and admits the heavy case. Cheap
locals were borderline around 13,000 units; the default is intentionally more
conservative and leaves some profitable computation sequential. It is easy
to lower and is a host-specific starting point, not a universal crossover.

## 13. Runtime formula and ordering

```text
static safety and callback identity
  -> source/executor workers and native-range/configuration guards
  -> actual positive trip count
  -> total_work = cost_per_iteration.saturating_mul(iterations)
  -> total_work >= threshold
  -> successful worker-pool acquisition and parallel execution
```

Trip count uses `i128` subtraction before converting the positive full `i64`
range difference to `u64`. Empty/reversed ranges perform no work. The added
profitability calculation has one multiplication and comparison, no allocation,
lock, configuration string lookup or recursive IR walk. The existing safety
side-table lookup and executor guards remain. Unsafe loops always stay serial.

## 14. Below-threshold evidence

`work_threshold_selects_real_workers_only_at_or_above_boundary` uses the same
compiled safe array transform with runtime-evaluated upper bound and thresholds
above, at and below its exact total. Above the total, observations are empty,
no pool starts, and every output equals forced sequential execution. The safety
proof remains independent. Other tests cover the default, high source override
and cached-but-idle workers after a threshold change.

## 15. Actual above-threshold threads

At equality and below the total, that test observes all 257 iteration numbers
exactly once, at least two distinct worker IDs, no caller-thread ID, a started
pool and correct output. Tests preserve integer, array read/disjoint write,
nested block, conditional, pure native/regex and nested-computation semantics.
Existing failure-prefix tests continue to pass. Timing is never thread proof.

After compilation the boundary test changes the native cost to `u64::MAX`;
the original decisions persist, proving invocation scheduling consumes the
cached summary. A full signed-bound runtime test also saturates at `u64::MAX`
and dispatches safely, then stops at the same first deterministic error as
sequential execution. Threshold-only callback tests protect extension visibility
and callback revision invalidation.

## 16. Regression results

Before edits, `cargo test-all` passed 917 language-library tests, five CLI
integration tests, 44 runner tests, one doctest and 550 language cases. The
parallel-specific Rust run passed 24 tests. Existing release benchmarks ran.

Final validation:

- `cargo test-all`: 949 language-library tests, five CLI integration tests,
  44 runner tests, one doctest and 554 `.eckt` cases pass; zero failures.
- `cargo test -p eck-lang parallel_execution`: 32 tests pass.
- `cargo test -p eck-lang analysis::work`: 14 tests pass.
- `cargo test-all -- testing/use-cases/language/automatic-parallelization`:
  six cases pass, including all four new source configuration cases.
- `cargo check -p eck-lang`, targeted rustfmt, and `git diff --check`: pass.

The estimator tests cover scalar/array work, casts/scales, native/nested calls,
finite dispatch maxima, additive blocks, conditional maxima, known/unknown
nested loops, reversed/empty ranges and arithmetic saturation.

## 17. Benchmark evidence

`cargo bench -p eck-lang --bench automatic_parallelization` passed with the
final default. The matrix compares the same compiled program with one worker,
four forced workers and four automatic workers, using two warmups and seven
rotating interleaved samples. Compilation/pool startup are excluded. The local
macOS host was previously documented as Apple M2 Max; Rust is 1.98.0.

| Body | Iterations | Work | Sequential ms | Forced ms | Automatic ms | Choice |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Cheap | 10 | 130 | 0.002541 | 0.022959 | 0.003000 | Sequential |
| Cheap | 10,000 | 130,000 | 0.563208 | 0.228084 | 0.562542 | Sequential |
| Cheap | 100,000 | 1,300,000 | 5.733833 | 1.943708 | 2.076625 | Parallel |
| Heavy | 10 | 7,530 | 0.025417 | 0.040000 | 0.026750 | Sequential |
| Heavy | 1,000 | 753,000 | 2.190958 | 0.701708 | 0.741542 | Parallel |
| Heavy | 1,000,000 | 753,000,000 | 2171.619625 | 565.950167 | 567.892833 | Parallel |
| Indexed write | 10,000 | 590,000 | 1.994084 | 2.542958 | 2.045750 | Sequential |

The two local-only bodies span 10 to one million iterations, including 30 and
300 to expose crossover. Bounded indexed writes span 10 to 10,000. Ten times
identical iterations gives exactly ten times estimated work; the heavy body
costs 753 versus 13 for cheap work. The default selects different strategies
at 1,000 iterations because their work differs.

Existing source-core comparisons also pass: at one million iterations,
one/four/ten workers measured 2176.485125 / 544.769250 / 313.502916 ms; at four
million, 8467.693959 / 2174.771375 / 1263.378500 ms. Those correspond to four-worker
speedups of 3.995x/3.894x and ten-worker speedups of 6.942x/6.702x.

Complete final, source-core and pre-change data are saved in the three
`work-cost-*.csv` files named above. Pre-change and final runs are not paired
or interleaved with one another; absolute changes cannot isolate host load or
code generation. No general old-versus-new execution speed claim is made.

## 18. Future-only artifact

`TODO/Desired/adaptive-parallel-cost-learning.md` describes stable versioned
region identity, warm-up counts, an exponential moving estimate, outlier
resistance, static cold start, gradual blending with compatible units, machine
locality, safety priority, extremely low sampling overhead, bounded concurrent
state, optional persistence and adaptive-versus-static benchmarks. No runtime
measurement, learning or persistence is implemented.

## 19. Known limitations

One static scalar cannot model input-length-dependent natives, serial replay,
memory bandwidth, worker topology or host load accurately. Finite dispatch can
overestimate ordinary paths because it prices possible widened representations.
Unknown inner-loop work is a fixed heuristic. Eligible large array workloads
can still be slower in parallel. Existing safety limits, including sequential
outer nested loops, maps, reductions and iterable sources, remain intact.
There is no normalized magnitude, runtime profiling, adaptive threshold, SIMD
or hardware cost model. Source threshold values are limited to signed `i64`;
internal work and executor thresholds can span all `u64` values.

## 20. Recommended next improvement

Calibrate the same single-scalar model on more indexed-write and native-input
sizes across worker budgets and machines. In particular, quantify the replay
cost that a threshold alone cannot distinguish from useful worker computation.
Only after that evidence, consider the future low-overhead learning design in
the desired-feature document; dependency safety must always remain independent.
