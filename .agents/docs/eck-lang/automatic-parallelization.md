# Automatic parallelization

ECK analyzes resolved typed IR after compilation and stores the result in
`TypedProgram.execution_analysis()`. The analysis is a side table keyed by source
loop spans; it does not change source syntax or rewrite the IR. For each loop,
it records resolved reads and writes, external effects, determinism, possible
failure, loop-carried dependencies, and either a sequential decision with
reasons or an `IndependentIterations` decision with the captured array slots
that workers may journal and the captured slots they need to read. Range entries
also retain a static `cost_per_iteration` summary prepared during compilation.

The compiled statement tree and proof have immutable public accessors.
`TypedProgram.statements_mut()` invalidates the proof before exposing a rewrite;
`TypedProgram.from_parts(...)` constructs manual IR with sequential execution.
A program compiled against one registry callback revision falls back to
sequential execution if defaults, index extraction, in-place operators or type
configuration hooks change afterward. The proof cannot be reused by a
replacement body that happens to have the same source span.

The analysis reasons about resolved binding identities and local slots, not
variable spelling. A loop is eligible only when its iterations can run
independently under the runtime's snapshot and journal rules. Read-after-write,
write-after-read, and write-after-write dependencies across iterations are
retained as evidence. Exact integer affine indices of the form `a * i + b` are
recognized. A captured array may be written when its index is affine in the
loop variable with nonzero coefficient. Reads and writes to the same captured
array may coexist when they use the identical affine index, so each iteration
reads its own element. Distinct captured arrays can be handled independently.

For adaptive arrays whose finite flow facts contain only plain signed values,
the compiler retains every widening width but removes unobserved subtype
alternatives from indexed-read dispatch. This allows ordinary `int[]` transforms
without inventing hypothetical qualified inputs. Qualified and fixed-width
array dispatch keep their existing conservative contracts.

## Native function effects

Native functions carry a `FunctionEffectSummary`: purity, determinism,
whether the call may fail, and its category of external effect. Registrations
without an explicit summary receive `UNKNOWN`, which is conservative and
prevents parallel execution when called in a loop. Extensions can declare a
summary with the `register_*_with_effect_summary` registry methods. Such a
declaration is part of the safety contract: it must accurately describe the
function's behavior.

Built-in String transformations are pure and deterministic, though some may
fail. The generic `string(value)` conversion remains unknown because extension
formatters may execute. `print` is an external write. `CSV.read` is an
external read and nondeterministic. Calls with external effects, unknown
effects, or nondeterministic results make the loop sequential.

## Static work and profitability

`parallel-safe` means the resolved safety proof permits concurrent execution.
`parallel-selected` means this invocation also reaches the work threshold,
passes runtime guards, and successfully obtains available workers. Cost never
changes safety or authorizes an unsafe region.

`analysis/work.rs` walks the resolved statement tree once after effect analysis.
Each nested body contributes its computed result to its parent without being
revisited. The only scheduling summary is `LoopAnalysis.cost_per_iteration`, a
`WorkCost` containing nonnegative `u64` relative units. Units are not time,
cycles, probabilities, or normalized magnitude. Addition and trip-count
multiplication saturate at `u64::MAX`; enormous workloads cannot wrap into
cheap work. Runtime never invokes the estimator or prices individual iterations.

The centralized V1 classes are:

| Class | Units | Initial uses |
| --- | ---: | --- |
| `Trivial` | 1 | Literal, local read, binding/assignment bookkeeping, break/continue |
| `Low` | 5 | Boolean operation, ordinary comparison, addition/subtraction, range bookkeeping |
| `Medium` | 10 | Array access/store/end intrinsic, multiplication, simple base cast |
| `High` | 30 | Division/remainder, decimal/bigint arithmetic and comparison, nonidentity subtype conversion, unspecified native cost |
| `VeryHigh` | 100 | Power, regex replacement, String repeat, unknown inner-loop surcharge |

Arithmetic reads resolved operator descriptors and operand types, including
compiled operand scales and relative adjustments. Each scale step includes its
literal factor and resolved operation. Decimal/bigint operations use High;
power uses VeryHigh. Finite dispatch uses the maximum candidate cost rather
than adding mutually exclusive candidates. Identity conversion costs Trivial,
a base cast Medium, and nonidentity scaling High plus its compiled scale steps.
Unsupported/open operations receive fallback estimates without changing their
safety rejection.

Statements and lexical blocks add child costs with no block surcharge. Empty
blocks cost zero. A declaration/assignment adds Trivial to its expression; an
indexed store adds Medium to its index and value computations. Boolean
short-circuit expressions conservatively include both operands. An `if` costs
its condition plus the maximum complete branch cost; a missing `else` costs
zero. No branch probability or runtime profiling is involved.

A range iteration costs its body plus five bookkeeping units. For a nested
range with already parsed signed literal bounds determining `N`, the statement
cost is bounds evaluation plus five setup units plus `N * (body_cost + 5)`.
Empty/reversed bounds add no body work. This composes recursively. Symbolic
bounds remain unknown: estimate one body iteration including bookkeeping plus
a VeryHigh (100-unit) surcharge, in addition to bounds and setup. Unknown
`while`/iterable regions similarly include one condition/source and body plus
100 units. This bounded heuristic is not a guaranteed upper bound. The safety
analyzer still rejects containing nested loops and handles eligible inner loops
separately; work summaries never broaden eligibility.

### Native cost summaries

`FunctionDescriptor.work_cost` is separate from `FunctionEffectSummary`.
Existing registration APIs default to High (30). Extensions can call
`Registry::set_function_work_cost(id, WorkCost)` to attach an exact nonnegative
cached summary; `CostClass::work_cost()` supplies the central classes. The
estimator reads the dense resolved descriptor at compile time. Later metadata
edits affect future compilations, not costs already stored in a compiled plan.
Unknown purity remains sequential regardless of estimated cost.

The current pure built-ins are String transformations. Trim helpers use Medium;
case conversion, padding, whitespace normalization, and literal replace/remove
use High; repeat and regex replacement use VeryHigh. Input-length prediction
is outside V1. Generic formatting, print and CSV retain their existing effects.
There is no user-defined function-body IR to analyze today. The exact cached
`WorkCost` field can hold future transitive summaries without rewalking callees
at each call site.

## Runtime behavior

`Executor` and `ExecutionOptions { workers, parallelization_threshold }`
provide a reusable Rayon worker pool with source-overridable settings. The
worker default uses the available CPU count. The default threshold is
**750,000 relative units**, shared by executor defaults and the source level-50
mapping through `analysis::DEFAULT_PARALLELIZATION_THRESHOLD`. The source default
is `parallelization.level: 50`. These are profitability settings, not safety proofs.

After safety and native-range guards, runtime derives the actual ascending trip
count from evaluated bounds with `i128` subtraction. A positive plain `i64`
range fits `u64`, including the full signed-bound difference. Once per invocation:

```text
total_work = cost_per_iteration.saturating_mul(actual_iteration_count)
parallel_selected = total_work >= configured_threshold
```

The profitability calculation adds no recursive walk, allocation, lock or
configuration name lookup; it reuses the existing source-span safety lookup.
A loop runs sequentially below threshold, for empty/reversed ranges, with fewer
than two workers, on pool startup failure, or when runtime guards do not match. Parallel execution currently requires a plain built-in `int` range with
`i64` bounds, unchanged non-scheduling configuration, and an identity integer
result transform. A scheduling override retains fast builtin integer execution;
extension result transformers still receive the override and are never skipped
merely because only scheduling settings changed.

### Source parallelization configuration

Both user controls live under `parallelization` in a root-level directive:

```eck
@config {
    parallelization: {
        cores: 4
        level: 50
    }
}
```

Configuration names accept identifiers or quoted strings. Entries are separated
by newlines; directives remain restricted to the program root. The registered
paths are `parallelization.cores` and `parallelization.level`. The former root
`cores` and raw source `parallelization.threshold` paths are rejected; migrate
worker settings into this object and replace raw thresholds with a level.

`cores` sets the worker budget. It accepts nonnegative integers, `null`, or
`None`. Values zero, one, `null`, and `None` select sequential execution and
never start a pool; `null` is parsed as the existing `None` configuration value.
Values above one select that many workers, including when the executor default
is one. Core counts are independent of the 0–100 level scale: `cores: 100000`
is valid, with no artificial 100-core or 100,000-core cap. Integer counts must
fit the platform worker-count representation. Negative, fractional, and other
symbolic values are rejected at compile time. Omission preserves the executor
budget, normally the available CPU count. Choosing more cores does not make an
unsafe loop parallel or bypass the work threshold.

`level` is an integer from **0 through 100**, with a default of **50**. It controls
how much estimated work is needed before using the parallel path. Higher levels
admit progressively smaller workloads. It is a deterministic control, not a
percentage probability, percentage of cores, or percentage of parallelized
loops. Write `level: 60` for level 60 on the 0–100 scale; no `%` suffix is required.
Negative, fractional, out-of-range, enum-like/nonfinite values, `null`, and `None`
are rejected before execution.

| Level | Internal minimum work | Meaning |
| ---: | ---: | --- |
| 0 | Disabled | Always sequential, including saturated work estimates |
| 1 | 22,800,000 | Requires very large workloads |
| 25 | 4,500,000 | Conservative |
| 50 | 750,000 | Default, preserving the previous default decision |
| 75 | 140,625 | Includes smaller workloads |
| 99 | 25,781 | Includes still smaller workloads |
| 100 | 0 | Every nonempty safe supported range that passes runtime guards |

Intermediate levels use integer linear interpolation between decadal anchors;
each ten-level increase halves the anchor threshold around level 50. For a
level `L` from 1 through 99, let `bucket = L / 10` and `offset = L % 10` using
integer division. With `D = 750000`, the threshold is rounded down:

```text
bucket <= 5: D * (20 - offset) * 2^(5 - bucket) / 20
bucket > 5:  D * (20 - offset) / (20 * 2^(bucket - 5))
```

Level 0 is an explicit disabled state, not a maximum integer threshold. This
prevents saturated work from qualifying. Level 100 maps to zero; even singleton
ranges may use the worker path. Empty/reversed ranges perform no iterations.
Safety proofs, native range guards, identity transforms, at least two workers
and successful pool startup remain required at every level, including 100.

A later `cores: null` or `cores: None` directive disables workers previously
selected by a numeric core count; it preserves the active level. Selecting a
numeric count above one later re-enables the worker budget subject to the same
safety and profitability checks.

The compiler converts each explicit level to its internal threshold once while
building a configuration override. Runtime consumes typed cached fields with
no configuration name lookup or level conversion per loop iteration. Both
scheduling leaves retain built-in identity fast paths while remaining visible
to extension callbacks through the full configuration map.
`RuntimeConfiguration.uses_initial_values()` becomes false for every nonempty
override. Extension result transformers still run; other value-setting changes
retain the conservative sequential fallback.

Omission preserves the executor defaults; the raw Rust field
`ExecutionOptions.parallelization_threshold: u64` remains available for tests
and benchmarks. Explicit source levels take precedence over this field. Later
directives affect subsequent loops; empty or worker-only directives retain the
active level, including disabled level 0. Level-only directives retain the
worker budget. Raising the level cannot make previously admitted work fail the
profitability check. A one-worker budget remains sequential even at level 100.

The executor caches its most recent parallel pool; sequential sections leave it
intact, so switching `1 -> 4 -> 1 -> 4` workers or `100 -> 0 -> 100` levels reuses
workers. Switching between different parallel budgets replaces the one cached
pool, bounding retained resources. Concurrent invocations retain ownership of
active pools until their workers finish. Startup failure runs sequentially.

Tests and benchmarks use `ExecutionOptions { workers: 4,
parallelization_threshold: 0 }` for forced safe parallel execution and a
one-worker executor for sequential comparison of the same compiled program.
Explicit source worker settings take precedence over executor worker defaults.

Workers share immutable IR and registry data and receive private local values
and configuration state. Captured array writes are journaled; reads in the
same iteration see that iteration's journaled writes. Work is bounded to
batches of at most 1,024 iterations per configured worker. After a batch
finishes, writes are committed in logical iteration order. If an iteration
fails, its completed writes are retained, later speculative writes are
discarded, and the error from the lowest failing logical iteration is
reported. Cancellation is cooperative and does not skip earlier iterations
that could fail. Thus serial commit preserves the observable write prefix and
error ordering while work within a safe batch runs concurrently.

The test-only iteration observer records iteration numbers and thread IDs to
verify actual distribution. Tests can set an explicit worker count, including
on single-CPU CI hosts.

## Current limits

The analysis is deliberately conservative. These cases remain sequential:

- mutation of a captured scalar, maps, and mutation of an array created inside
  a loop body;
- non-affine, repeated, modulo, or otherwise unproven writes, and conflicting
  same-container accesses such as shifted read/write indices;
- reading a whole captured array that the loop writes, or aliases created
  inside the loop;
- unknown or unsupported callback operations, unsupported constructs and
  range plans, and loops with external effects or nondeterministic calls;
- iterable/source loops, `break`, and nested loops at the outer level.

`continue` is compatible with parallel execution because it ends only the
current iteration; writes already made by that iteration remain in its ordered
prefix. A nested loop is analyzed separately and can be eligible when executed
independently, while the containing loop remains sequential. The runtime does
not currently parallelize local-array mutation. No map, source, nested-loop,
reduction, or broader index support is implied beyond the cases above.

## Measurement

The in-process benchmark is documented under
`testing/benchmarks/language/automatic-parallelization/README.md` and runs with
`cargo bench -p eck-lang --bench automatic_parallelization`. It reports serial,
forced parallel and automatically scheduled medians across increasing sizes
for cheap and arithmetic-heavy bodies, plus bounded indexed writes. Each
shape/size uses one compiled program, two warmups and seven rotating interleaved
samples; compilation and initial pool startup are excluded. Small-loop overhead is measured independently of the profitability
threshold; timing is never used to prove correctness or multithreading. The
heavy source-configured comparison compiles otherwise identical programs with
`parallelization.cores: 1`, `parallelization.cores: 4` and
`parallelization.cores: 10`, uses ordinary executor defaults,
and reports seven interleaved rounds with rotating budget order after two
warmups for one and four million iterations. Each budget retains its own default
executor to exclude pool replacement when switching between parallel budgets.
Compilation and initial pool startup are excluded from these timings.


The initial 750,000-unit default is informed by the local three-shape matrix.
Forced execution lost to sequential for 10,000 indexed writes (590,000 units)
and won substantially for 1,000 heavy iterations (753,000 units). Cheap locals
were borderline around 13,000 units and profitable at larger sizes. The default
avoids the measured replay loss while admitting larger computation. This is a
conservative host-specific starting point, not a universal crossover guarantee.
The cost table prices potential widening through finite dispatch. String
lengths, serial journal replay, memory bandwidth, worker budgets and host load
are not modeled separately. A larger array workload can still qualify while
losing performance. Current measurements and CSV live in the benchmark README.

Future runtime learning is documented only in
`TODO/Desired/adaptive-parallel-cost-learning.md`. No timers, adaptive thresholds,
per-node profiling, or learned state are implemented by this feature.
