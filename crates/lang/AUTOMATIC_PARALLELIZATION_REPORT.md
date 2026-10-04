# Automatic parallelization development report

Historical report of the original iteration-threshold implementation. Current
configuration uses `parallelization { cores, level }` and static work costs; see
`STATIC_WORK_COST_REPORT.md` and the authoritative automatic-parallelization
document for the current behavior. Measurements below remain historical.

Implemented on 3 October 2026. Language implementation remains entirely within
`crates/lang`; no top-level crate or source-level concurrency syntax was added.
Existing unrelated workspace changes were preserved. No commit was created.
Suggested commit subject: `feat(lang): automatically parallelize independent range loops`.

## Architecture and files

The existing parser, AST, resolved semantic registry and compiler remain the
language pipeline. After the compiler finishes resolving the complete
`TypedProgram`, `analysis::analyze` creates an owned execution side table. The
runtime consumes the retained loop proof and applies separate profitability and
resource guards before choosing serial or parallel execution.

| Owner | Added or changed files |
| --- | --- |
| Execution analysis | `src/analysis/mod.rs`, `effects.rs`, `resolved.rs`, `resolved/builtins.rs`, and all four matching `.tests.rs` files |
| Compile-time integration | `src/compiler/mod.rs`, `src/ir/program.rs` |
| Adaptive indexed-read dispatch | `src/compiler/expressions.rs`, `src/containers/array/compiler/access.rs`, `access.tests.rs` |
| Scheduler and execution | `src/runtime/runtime/parallel_execution.rs`, `parallel_execution.tests.rs`, `src/runtime/runtime.rs`, `runtime.tests.rs`, `src/runtime/mod.rs` |
| Array journal integration | `src/containers/array/runtime.rs` |
| Native effect contract | `src/semantic/descriptor.rs`, `src/semantic/registry/functions.rs`, `functions.tests.rs`, `src/semantic/mod.rs` |
| Callback proof invalidation | `src/semantic/registry/mod.rs`, `types.rs`, `operators.rs`, `comparisons.rs`, `configurations.rs` |
| Audited native registrations | `src/primitives/string/functions/mod.rs`, `src/primitives/string/mod.rs`, `src/std/io/mod.rs`, `src/connectors/csv/runtime.rs` |
| Crate facade and dependency | `src/lib.rs`, `Cargo.toml`; Rayon 1.12 and its lockfile dependency graph |
| Benchmark harness | `benches/automatic_parallelization.rs` |

Outside the crate, documentation is in
`.agents/docs/eck-lang/automatic-parallelization.md`, indexed by `AGENTS.md`.
Benchmark input, instructions and measured results are under
`testing/benchmarks/language/automatic-parallelization/`. The new CLI case is
`testing/use-cases/language/automatic-parallelization/disjoint_array_transform.eckt`.

`TypedProgram` exposes immutable `statements()`, `bindings()`,
`local_slot_count()` and `execution_analysis()` accessors. Public struct-field
mutation was replaced because it could preserve an obsolete safety proof.
`statements_mut()` invalidates the proof before permitting a rewrite;
`from_parts(...)` constructs manually resolved IR with sequential execution.
This is a Rust API migration; ECK source syntax and the `execute` entry point
remain unchanged.

## Effect, purity and dependency models

`EffectSummary` retains located reads/writes, external effects, determinism,
possible failure and structured rejection reasons. Resources identify resolved
`BindingId` and `LocalVariableSlot`, individual indexed array elements, or maps.
Blocks combine their complete children, and branches conservatively join every
path. Pure local state does not become a capture merely because source names
shadow an outer declaration.

`FunctionEffectSummary` records `FunctionPurity`, `FunctionDeterminism`,
`FunctionExternalEffect` and `may_fail`. Legacy registration defaults to
`UNKNOWN`; explicit `register_*_with_effect_summary` APIs provide the trusted
native contract. Nested calls compose the arguments' effects with each resolved
function descriptor. A pure declaration must be accurate and deterministic,
with no external-state access, to qualify.

String transformations are declared pure and deterministic; their default
parser dependencies are independently verified. Generic `string(value)` stays
unknown because it invokes extension formatters. `print` declares an external
write, and `CSV.read` declares an external read. Operators, comparisons, index
extractors, result hooks and boolean evaluators are checked against an actual
cached builtin callback inventory; familiar names alone are insufficient.
Registry identity and callback revision guard the proof against later changes, including newly installed types, aliases and operation callbacks.

The analyzer retains loop-carried read-after-write, write-after-read and
write-after-write access pairs. Scalar accumulation remains sequential with
binding and dependency evidence suitable for later reduction analysis.
`Parallelism` is either `Sequential { reasons }` or
`IndependentIterations { written_array_slots, capture_read_slots }`.

## Loop eligibility and indices

Independent range iterations may read immutable captures, compute arbitrary
compositional pure local blocks, branch, call declared pure native functions,
use `continue`, and journal proven disjoint captured array stores. Existing COW
aliases created before the loop remain distinct language values, so a write to
one binding cannot alter another binding's input snapshot.

The index proof recognizes exact integer `a * i + b`, including `i`,
`i + constant`, `i - constant`, `constant * i`, `i * constant`,
`constant * i + constant` and `constant * i - constant`. Negative coefficients
also work. A write requires a nonzero coefficient. Reads and writes to the same
container must use the identical affine form; differing forms are conservatively
conflicting even when a stronger solver might establish disjointness.

Repeated/constant, modulo, nonlinear and unknown-function indices stay
sequential. Captured scalar mutation, maps, array end operations, whole reads of
written arrays, aliases created inside the loop, local-array mutation, unknown
or effectful callbacks, open operation caches, `break`, sources and unsupported
conversions remain sequential. Outer nested loops stay sequential; separately
eligible inner loops may run in parallel. Reductions are not implemented.

Adaptive arrays with flow-proven plain signed elements retain all widening
widths but prune impossible qualified subtype candidates from indexed-read
plans. This supports ordinary `output[i] = input[i] * 2` without narrowing the
adaptive storage contract or changing qualified/fixed-array dispatch behavior.

## Workers, scheduling and deterministic failure

`Executor` owns a lazy, bounded reusable Rayon pool. Rayon provides sound
borrowing and worker lifecycle management without a custom unsafe pool. The
shared production executor respects available CPU resources; tests can request
four workers even when CI reports one CPU. `ExecutionOptions` separates worker
count and minimum parallel range length from static safety. The default
threshold is 50,000 iterations. Tiny loops do not start workers, and pool startup
failure falls back to sequential execution.

The initial concurrent backend handles plain default builtin `int`/`i64`
bounds under unchanged non-scheduling configuration with identity integer transforms.
Other invocations retain the existing range executor. Analysis, affinity and
purity are not recomputed per iteration.

Source `@config { "cores": 4 }` selects the worker budget for following eligible
loops. Signed values at or below one select serial execution; omission preserves
the executor default. Quoted configuration keys are supported. The normalized
worker budget is decoded when compiling the override, never looked up by name
per iteration. The most recent parallel pool is retained across serial sections;
changing parallel budgets replaces that one cached pool. Active invocations own
an Arc so concurrent pool changes cannot destroy their workers.

A scheduling-only override retains builtin identity fast paths while preserving
extension semantics: `uses_initial_values()` becomes false for every override,
and configured result hooks still receive the complete configuration. Direct
binary execution applies transformations to the actual result type after integer
promotion, rather than skipping a promoted type's hook based on the input plan.

Workers share immutable registry/IR and the already-prepared body plan. Each
owns its locals, configuration, loop control, value stack and array-write journal.
There is no global mutex around iteration computation. Open dispatch caches are
excluded from eligible bodies. Worker runtimes disable nested dispatch.

Broadcast chunks cover each range exactly once. Batches bound speculation to
1,024 iterations per configured worker. Array reads see that iteration's latest
journaled store. Workers never mutate caller arrays; the caller joins the batch
and applies stores in logical iteration/source order. A shared atomic lowest
failure index permits cooperative cancellation without skipping earlier work.
When multiple iterations fail, the lowest logical iteration's error wins.
Successful earlier writes and completed stores within that failing iteration
are committed; all later speculative writes are discarded. No worker continues
mutating observable caller state after the failure is returned.

## Validation

- **36 analysis tests** cover resources, composed effects, affine forms,
  dependencies, captures, aliases, control flow, duplicate spans, pure/unknown/
  impure calls and counterfeit or changed extension callbacks.
- **24 runtime parallel tests** compare sequential and forced parallel results,
  array reads/writes, branching, continue, aliases, affine strides, negative and
  empty ranges, nested-loop limits, native calls and exact failure prefixes.
- The test-only observer records logical iteration and `ThreadId`; a four-worker
  array transform proves all iterations occur exactly once on multiple worker
  identities and excludes the calling thread. Unsafe loops never dispatch the
  observer. Correctness never depends on elapsed time.
- Stress exercises 12 executions of 4,133 writes, including full batches and
  tails, on one pool and verifies reuse of exactly four worker identities.
  Twenty error repetitions compare deterministic errors and partial state.
- Two native metadata tests audit default/explicit and builtin summaries; two
  array compiler tests preserve width dispatch while checking subtype pruning.
- The stale-IR-proof regression rewrites a formerly independent loop into an
  accumulator, invalidates analysis, and verifies the correct serial result.
- `cargo test-all`: **968 Rust tests passed** (917 language unit tests, five CLI
  integration tests, 45 runner tests and one doc test); **550 CLI use cases
  passed**; zero failures. The initial 548 use cases also passed before changes.
- `cargo fmt --package eck-lang` and `git diff --check` pass. Clippy completes;
  unrelated existing warnings remain.

## Benchmarks

`cargo bench -p eck-lang --bench automatic_parallelization` measures current
workspace source in release mode, excluding parsing, analysis and initial pool
startup. Twelve dependent integer stages execute per iteration; iterations are
independent. Each mode receives two warmups and seven timed samples. The local
macOS host used four workers. Units are median milliseconds per full execution.

| Iterations | Sequential | Forced parallel | Automatic | Automatic speedup |
| ---: | ---: | ---: | ---: | ---: |
| 10 | 0.035333 | 0.064250 | 0.034000 | 1.039× |
| 100 | 0.262083 | 0.099125 | 0.270750 | 0.968× |
| 1,000 | 2.218625 | 0.582167 | 2.151875 | 1.031× |
| 10,000 | 21.478958 | 5.555625 | 21.790875 | 0.986× |
| 100,000 | 215.068250 | 55.173542 | 55.202000 | 3.896× |
| 1,000,000 | 2126.600375 | 545.986667 | 545.905000 | 3.896× |

Forced scheduling exposes startup-independent overhead at ten iterations and a
crossover between ten and one hundred for this body. The conservative default
threshold leaves smaller cases sequential and schedules the two largest cases.
Measurements are host/workload-specific medians, not universal guarantees or
statistical confidence claims.


## Source-configured cores: measured result — 3 October 2026

Release build on Apple M2 Max (12 logical CPUs), using ordinary executor
settings. Each iteration executes twelve dependent integer arithmetic stages.
The same source is compiled with only the `cores` value changed. Two warmups
per variant precede seven interleaved measurement pairs; compilation and initial
pool startup are excluded. Times are median milliseconds per complete execution.

| Iterations | `cores: 1` ms | `cores: 4` ms | Speedup | Execution time reduction |
| ---: | ---: | ---: | ---: | ---: |
| 1,000,000 | 2076.481250 | 537.932958 | 3.860× | 74.09% |
| 4,000,000 | 8273.493750 | 2145.633375 | 3.856× | 74.07% |

These timings measure pure local arithmetic on this host. They do not measure
array-write replay, establish a statistical confidence interval, or predict
all workloads. Worker distribution and configuration behavior are verified
separately with deterministic runtime tests.


## One, four and ten source workers: measured result — 3 October 2026

Fresh release measurements on Apple M2 Max (8 performance and 4 efficiency
cores). The one-worker and four-worker baselines were rerun together with ten
workers on the same twelve-stage integer workload. Each source budget has its
own default executor, two untimed warmups and seven timed executions. Order
rotates over seven interleaved rounds. Compilation, initial pool construction
and pool replacement are excluded; the runtime implementation is unchanged.

| Iterations | `cores: 1` ms | `cores: 4` ms | `cores: 10` ms | 4 vs 1 | 10 vs 1 | 10 vs 4 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000,000 | 2106.902209 | 537.259750 | 313.198958 | 3.922× | 6.727× | 1.715× |
| 4,000,000 | 8450.507959 | 2160.326541 | 1239.210167 | 3.912× | 6.819× | 1.743× |

Ten workers reduce execution time by 41.70% relative to four workers at one
million iterations, and by 42.64% at four million. Scaling is sublinear; these
measurements do not isolate CPU heterogeneity from runtime scheduling overhead.
Results apply to this arithmetic workload and host, not all ECK programs.
The previous two-budget measurements above remain available for comparison.

## Known limitations

Only range loops have a concurrent backend. There is no reduction, streaming,
map, DataFrame, distributed or SIMD execution. Affine proofs do not infer
relations through arbitrary local aliases or solve different affine regions.
Non-scheduling configuration overrides and non-default integer bounds use sequential fallback.
Native purity remains a trusted extension-author contract. Ordered array replay
is serial and consumes bounded journal memory, so array-write speedups depend on
how much independent CPU work surrounds each store. The benchmark measures pure
local arithmetic and does not establish array-write speedup. The fixed range
length threshold cannot predict every body's cost. These are explicit V1 limits;
none weakens the sequential fallback or deterministic language semantics.
