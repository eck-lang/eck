# ToDo — Array implementation audit

Audit date: 2026-09-13. Base commit: `aefb96a9e35bbfe4494dd07af9bf5dc652bd65e9`.

**This report audits the working tree, including staged, modified, and untracked files; it is not a review of HEAD alone.** Existing implementation changes were not reverted or repaired. Re-read the referenced functions before applying fixes, since another agent may change them after this audit.

The implementation covers the basic documented syntax, but is not yet semantically sound. The highest-priority defects silently produce incorrect quantities, allow values outside an array's declared contract, and lose actual representation information. Passing the current tests does not establish correctness.

## Instructions for the implementing agent

- Read `AGENTS.md` and its relevant authoritative `.agents/docs/` documents first. Keep artifacts and code comments in English.
- Complete correctness work before optimizing paths that currently produce incorrect results. Preserve subtype, precision, configuration, diagnostics, evaluation ordering, and copy semantics.
- Use the task IDs below as individually reviewable units. Check an item only after its acceptance criteria pass.
- Fix shared causes across initialization, indexed assignment, inference, extracted bindings, and execution. Do not add isolated special cases for the reproducer strings.
- Use sibling `*.tests.rs` files for unit tests and ownership-aligned `.eckt` files for CLI regressions. Probe exact CLI output before recording diagnostics.
- Do not change expected outputs just to match a bug. Distinguish documented requirements from the explicitly identified specification decisions below.
- If changing `.agents/docs/`, review and synchronize the `AGENTS.md` index as instructed. This audit itself does not change those documents.
- Follow `.agents/docs/eck-lang/performance.md`: pre-resolve known work, measure hot paths, and do not sacrifice execution speed just to shorten code.

## Evidence and coverage

Commands executed successfully against this working tree:

| Check | Result |
|---|---|
| `cargo build -p eck-cli` | Passed |
| `cargo build --release -p eck-cli` | Passed |
| `cargo test --workspace` | 782 Rust tests passed, zero failures across reported suites |
| `cargo test-all -- testing/use-cases/language/arrays` | 57 passed, zero failures |
| `cargo test-all` | 408 passed, zero failures |
| Additional CLI probes | 32 programs compared between debug and release; identical exit codes, stdout, and stderr |

The additional probes were temporary files, not added to the permanent regression suite. Self-contained reproductions of the actionable failures are included below. Run each fenced ECK program as a separate `.eck` file with `./target/debug/eck path.eck`; expected output means the required result after fixing the issue, unless explicitly labeled a specification decision.

Inspected implementation ownership:

| Layer | Files / responsibilities |
|---|---|
| Syntax/parser | `crates/syntax/src/ast/{expression,statement}.rs`; `crates/parser/src/parser/{expressions,statements}.rs`: literals, annotations, postfix indexing, indexed assignment |
| Compiler | `crates/compiler/src/compiler/arrays.rs`, `compiler.rs`, `compiler/{expressions,helpers,scopes}.rs`: contracts, inference, flow tracking, conversions, dispatch |
| IR | `crates/ir/src/{expression,statement}.rs`: array contract, element access, indexed stores, dynamic plans |
| Core | `crates/core/src/value.rs`, `descriptor.rs`, `registry/{types,subtypes}.rs`: storage, integer/index registration, dispatch slots |
| Primitives | Integer `mod.rs` and `value.rs` implementations, including fixed widths, unsigned widths, bigint and index extraction |
| Runtime | `crates/runtime/src/runtime.rs`, `runtime/{evaluation,loop_execution}.rs`: allocation, reads, writes, COW, scaling, casts, fallback execution |
| Tests/measurement | Compiler array tests, parser/runtime/core tests, all 57 array use cases, existing array benchmark and benchmark runner semantics |

### Documentation consistency matrix

| Requirement | Assessment |
|---|---|
| `let` / `const`, `[]`, `base[]`, `base<subtype>[]`, multiline literals | Basic forms work; parser unit coverage is missing (T14). |
| Empty arrays need an explicit element type | Covered and working. |
| Homogeneous declared base type | Direct incompatible literals are rejected; nullable and dynamically promoted elements bypass the contract (T02, T03, T08). |
| Adaptive `int` | Positive unconstrained annotated literal case works; negative, inferred, and constrained cases are incomplete (T06). |
| Fixed-width integer storage | Literal overflow is rejected; expression overflow is stored anyway (T02). |
| Unconstrained elements preserve their own subtypes | Works in many direct/conditional/dynamic tests; loop flow analysis is unsound (T01). |
| Constrained compatible subtype conversion | Works for common literal cases; dynamic source elements fail (T04); exactness differs by width (T05). |
| Extracted values behave like scalar values of the stored complete type | Not consistently true for dynamically widened indexes and unsigned elements (T07, T08). |
| Mutable bindings can be reassigned without changing type | Whole-array reassignment is explicitly rejected, conflicting with the general mutable-binding rule (T09). |
| Lexical bindings are released after their scope | Runtime slots retain array owners after scope exit, causing unnecessary COW and retained memory (T10). |
| Test ownership hierarchy | `testing/use-cases/language/arrays/` does not mirror the implementation tree (T14). |
| Resolve known work before execution | Partly implemented for indexes and binary plans; conversion scaling and casts still repeat work (T11). |

## Correctness tasks

### T01 — P1: Make array type flow sound across loops and abrupt exits

- [ ] Fix loop-carried element types, repeated conditions, and `break` / `continue` exit states.

**Evidence:** `crates/compiler/src/compiler.rs:206` compiles a while condition before its body and merges element records only afterward; `compile_for_statement` likewise compiles the body once from entry state. `arrays.rs:461`–`508` snapshots/merges records but does not compute a loop-entry invariant. `compile_block` continues processing statements after unconditional control transfer.

```eck
let a: int[] = [10mm]
for (i in 0..2) {
    print(a[0] + 1mm)
    a[0] = 2cm
}
```

Observed: `11mm`, then **`3mm`**. Required: `11mm`, then **`21mm`**. The second iteration uses an operation compiled assuming millimeters, even though the stored value is centimeters.

```eck
let a: int[] = [10mm]
let n = 0
while (a[0] < 15mm) {
    n = n + 1
    a[0] = 2cm
    if (n == 3) { break }
}
print(n)
```

Observed: **`3`**. Required: **`1`**. Without the defensive break, the stale comparison can keep this program running indefinitely.

```eck
let a: int[] = [10mm]
for (i in 0..1) {
    a[0] = 2cm
    break
    a[0] = 10mm
}
print(a[0] + 1mm)
```

Observed: **`3mm`**; required: **`21mm`**. Unreachable writes corrupt the apparent exit state.

**Implementation direction:** compute conservative write effects before compiling repeatedly executed reads/conditions, or use a proper fixed-point flow analysis. Merge actual reachable exits, including loop backedges, zero iterations, break and continue. Do not simply merge after producing IR that already contains stale plans. Track related extracted scalar bindings as well as array slots.

**Acceptance:** permanent tests for all three examples, continue before/after subtype mutation, conditional writes inside loops, nested loops, zero iterations, aliases, and extracted scalar values used on a later iteration. Preserve precise dispatch for provably stable elements.

### T02 — P1: Enforce element representation after evaluating expressions

- [x] Prevent fixed-width arrays from storing overflow-promoted values; apply the contract to both initializer elements and stores.

**Status:** Fixed in the working tree. The compiler resolves an array element
destination for every initializer element and indexed store, carries it into the
IR as `TypedExpressionKind::ElementStore`, and the runtime normalizes the
evaluated value to that representation before the array is touched or rejects it
without modifying the stored element; adaptive `int[]` destinations keep their
auto-widening semantics. Permanent cases live in
`testing/use-cases/language/arrays/element-constraints/`, with runtime unit tests
for normalization, rejection, and store atomicity.

**Evidence:** `arrays.rs:184` (`compile_array_element`) trusts `TypedExpression.output`, with early returns for matching static types. `runtime/evaluation.rs:240` and `runtime.rs:150` store evaluated values without validating the array contract. `TypedStatement::IndexedAssignment` carries no explicit element contract or prepared validation/coercion plan. Integer context executors can promote after compilation.

```eck
let a: int8[] = [127]
a[0] = a[0] + 1
print(a[0])
```

Observed: **`128`**, exit 0. The declared `int8[]` cannot represent this value. Also reproduced with `let a: int8[] = [127 + 1]`, `int8<mm>[] = [127mm + 1mm]`, and `int64[]` storing `9223372036854775807 + 1`.

**Implementation direction:** make the stored contract available wherever a value crosses an array boundary. Compile away checks for proven-safe values; otherwise perform a prepared checked conversion/validation before storing. Reject out-of-range results without modifying the original element. If a computation promotes temporarily but its final result fits, normalize to the declared representation according to the documented contract instead of rejecting solely because a temporary used a wider type.

**Acceptance:** initializer/store boundary tests for every registered fixed integer width; positive overflow, negative underflow, subtype-qualified results, dynamic reads, casts and arithmetic; assert payload/base identity as well as printed magnitude. Include an in-range result after temporary promotion.

### T03 — P1: Reject nullable elements before nullability information is lost

- [ ] Apply scalar/nullability validation in both declared and inferred literal paths, and indexed assignment.

**Evidence:** `compile_array_element` rejects nested arrays and void values but does not use the nullable-expression guard in `compiler/scopes.rs:160`. The inferred path similarly reads `output` without preserving/checking nullable binding metadata.

```eck
let x: int? = null
let a: int[] = [x]
print(a[0])
print(a[0] + 1)
```

Observed stdout: `null`; then runtime error **`invalid value representation for type int64`**. `let a = [x]` and `a[0] = x` also accept the null. A non-nullable scalar can then be inferred from that element without a nullability diagnostic.

**Acceptance:** reject potentially nullable elements before execution for declared initialization, inference and stores; retain legal non-null narrowing, e.g. constructing `[x]` inside `if (x != null)`. Test nullable values currently holding a number as well as null, so the rule does not depend on the current payload. Treat nullable containers and nullable elements as distinct features.

### T04 — P1: Use stored source subtypes for constrained element initialization and mutation

- [ ] Route dynamic source values through a pre-resolved constrained conversion plan.

**Evidence:** `arrays.rs:221` resolves a constrained conversion using `typed.output` directly. Unlike explicit `->to(...)` compilation in `expressions.rs:780`, it does not branch on `dynamic_complete_type()`.

```eck
let a: int[] = [2cm]
let i = 0
let b: int<mm>[] = [a[i]]
print(b[0])
```

Observed: compile error **`conversion from int64 to subtype millimeter is not defined`**. Required: **`20mm`**. The equivalent indexed store into an existing `b` fails the same way.

**Acceptance:** initialization and mutation from dynamic indexes, values extracted into bindings, and post-mutation sources; compatible runtime subtypes convert, incompatible ones produce a semantic conversion error. Cover adaptive and fixed-width destinations. Reuse the conversion planning contract, including T02/T05 validation, rather than duplicating a third conversion implementation.

### T05 — P1: Resolve inconsistent exactness of integer subtype conversion

- [ ] Make array conversion representability checks exact and consistent across integer widths; resolve the shared scalar conversion rule explicitly.

**Evidence:** `arrays.rs:774` (`literal_conversion_succeeds`) and `runtime/evaluation.rs:290` (`scale_magnitude`) choose fractional division only for the default integer representation. Other fixed integer widths divide using integer arithmetic. The literal checker immediately accepts a result if its base equals the declared base, after any fraction has already been discarded.

```eck
let a: int8<cm>[] = [15mm]
print(a[0])
```

Observed: **`1cm`**, exit 0. A variable source `let x: int8 = 15mm; let a: int8<cm>[] = [x]` behaves the same way. The existing `array_inexact_literal_element_conversion_rejected.eckt` instead requires `int<cm>[] = [5mm]` to be rejected because it has no exact integer representation.

**Important scope distinction:** `let x: int8 = 15mm; print(x->to(int8, cm))` also prints `1cm`. This is a shared conversion inconsistency, not evidence that an independent, array-only arithmetic rule should be invented. Array docs explicitly say extracted values use normal scalar conversion semantics. Reconcile the shared exact conversion policy with the existing rejection test, then enforce it at all array boundaries. Ordinary integer division need not change.

**Acceptance:** exact and inexact conversions for signed/unsigned widths and bigint, literals and variables, both directions and negative values; overflow during scale multiplication; explicit conversions compared with array insertion. No validation after irreversible truncation.

### T06 — P1: Complete adaptive integer literal handling and fix widening selection

- [ ] Support adaptive inference and qualified/negative literals consistently; choose an integer representation by semantic widening order rather than spelling.

**Evidence:** `arrays.rs:192` only retries when the destination is adaptive, has no subtype constraint, and the AST node is directly `Expression::Number`. Inference calls ordinary `compile_expression` without that fallback. `compile_widened_integer_literal` iterates `registered_type_names()`, which sorts names alphabetically: `bigint` is tried before `int128`, although the latter can represent many values beyond int64 without heap-backed bigint storage.

Each following declaration currently fails with an int64 literal range error:

```eck
let a: int[] = [-999999999999999999999999999999]
```

```eck
let a: int<mm>[] = [999999999999999999999999999999mm]
```

```eck
let a = [1, 999999999999999999999999999999999]
```

The positive annotated unconstrained equivalent passes. The documentation makes adaptive behavior a property of `int`, not of those particular AST shapes.

**Related shared defect:** `let a: int8[] = [-128]` rejects the positive token `128` before applying negation. The scalar `let x: int8 = -128` also fails. Repair negative-literal handling at the appropriate shared layer and preserve the valid signed minimum.

**Acceptance:** values on both sides of int64/int128 boundaries; beyond int128; constrained/unconstrained and inferred arrays; initializer/store paths; fixed arrays still reject genuine overflow. Verify selected runtime representations, not just printed values. Avoid broad retry-on-any-compilation-error behavior or dependence on extension naming order. Define how fixed-width source bindings affect inferred adaptive contracts, since comparing only to the default integer TypeId cannot preserve the `int` versus `int64` distinction.

### T07 — P1: Select the correct index extractor for dynamically widened values

- [ ] Make index extraction aware of possible runtime base representations without textual fallback.

**Evidence:** `arrays.rs:333` picks one extractor from the static base; `runtime/evaluation.rs:269` calls it even when an extracted adaptive element or its arithmetic result has widened. Runtime arithmetic has redispatch, but index extraction does not.

```eck
let a: int[] = [0, 999999999999999999999999999999]
let i = 1
let x = a[i] - a[i]
print(a[x])
```

Observed: **`invalid value representation for type int64`**. Required: **`0`**. Replacing both `a[i]` expressions with `a[1]` works, demonstrating that a semantically equivalent dynamic index loses representation information.

**Acceptance:** constant/dynamic reads and writes with int64, int128 and bigint results, all narrower/unsigned supported index types, negative values, oversized values and qualified values. Resolve the finite candidate extractors in the IR where possible; retain the plain-integer and bounds checks.

### T08 — P1: Model possible runtime base types, not just dynamic subtypes

- [ ] Remove the assumption that a single representative output base is a sufficient element contract for dynamic expressions.

**Evidence:** `dynamic_binary`/`dynamic_convert` build candidate tables indexed by subtype while fixing the source base. `dynamic_binary` chooses a representative result base (`arrays.rs:582`), even when outputs differ. `compile_array_element` accepts adaptive elements using the broad `is_integer_type` predicate, including unsigned types whose mixed operations differ from signed `int`.

```eck
let a: int[] = [15mm]
let i = 0
let b: int[] = [a[i]->to(cm)]
print(b[0])
```

Observed: **`1.50cm`** inside a declared **`int[]`**. The actual stored value is fractional, but the compiler allowed it based on the representative static base.

```eck
let x: uint8 = 1
let a: int[] = [x]
let i = 0
print(a[i] + 1)
```

Observed: **`invalid value representation for type int64`**. If unsigned values are not compatible with adaptive signed `int`, reject this at the array boundary. If they are accepted, preserve the scalar operation compatibility and report an undefined operation where appropriate; do not run an int64 executor on a uint8 payload.

**Acceptance:** enforce actual output contracts for dynamic conversions/arithmetic, and preserve each supported concrete base when compiling later arithmetic, comparison, calls, pipes and indexing. Add public-registry integration tests with non-default integer types/extension registration order. An unsupported semantic operation must never degrade into an internal representation error. Coordinate with T02/T07 instead of accumulating separate redispatch patches.

### T09 — P2: Reconcile whole-array assignment and typed copy initialization with documentation

- [ ] Support mutable whole-array reassignment with a stable contract, or explicitly resolve the documented scope before retaining a restriction.
- [ ] Accept compatible typed copy initialization, or document why it is intentionally excluded.

**Evidence:** `compiler.rs:470` unconditionally rejects whole-array assignment. `arrays.rs:103` requires every explicitly annotated array initializer to be a literal. Thus `let b = a` works while `let b: int[] = a` fails even for identical contracts.

```eck
let a: int[] = [1]
a = [2]
print(a[0])
```

Observed: `whole-array reassignment is not supported`. The general binding specification permits reassignment of `let` while preserving its declared/inferred type; no array exception is stated.

```eck
let a: int[] = [1]
let b: int[] = a
print(b[0])
```

Observed: `an array binding must be initialized with an array literal`. Expected under ordinary initializer compatibility: `1`.

**Specification decisions, not proven bugs:** nested arrays, nullable containers, array printing/equality, copy/value semantics, aliasing through const, array length changes on reassignment, and index-versus-RHS side-effect ordering are not fully defined in the array document. Existing rejection/COW tests must not substitute for that specification. Document decisions before broadening these features. The existing RHS-first store evaluation should be preserved until the language specifies otherwise.

**Acceptance:** documented assignment/copy rules; compatible and incompatible contracts, empty arrays, const rejection, aliases and flow-record invalidation after reassignment. Review `rejects_whole_array_reassignment` and its `.eckt` instead of treating them as authoritative evidence of intended behavior.

## Performance and lifetime tasks

### T10 — P1: Release expired local array owners to avoid repeated full copies

- [ ] Clear scope-local storage on all scope exits, including break/continue/error paths, without dropping live outer values.

**Evidence:** `runtime.rs:346` executes block statements but never clears their local slots. Slots live in `Runtime.local_values` until overwritten or execution ends. An inner alias therefore remains an Arc owner after its lexical scope has ended. `execute_indexed_assignment` detects sharing and clones the entire `Vec<Value>`.

```eck
let values: int[] = [0, 0, 0, 0]
for (iteration in 0..2000) {
    { let copy = values }
    values[0] = values[0] + 1
}
print(values[0])
```

The expired `copy` forces COW on every iteration. With N elements and K iterations this adds O(N × K) element copying instead of O(K) writes. This also retains arrays declared in scopes that will never execute again. Compiler element-type records for expired bindings are likewise not removed in `compile_block`.

**Exploratory measurement on this working tree:** arm64, release binary, 2,000 iterations, one warmup and five subprocess samples per case. Timed region includes startup, parsing, compilation and execution; build excluded; every run checked stdout `2000`. Arrays were literal zero lists of the indicated size.

| Elements | No inner alias, median | Expired inner alias, median |
|---|---:|---:|
| 8 | 4.695 ms | 5.124 ms |
| 10,000 | 7.154 ms | 54.731 ms |

These are local exploratory timings, not an isolated runtime benchmark or a measured speedup from a fix. Promote the reproducer to a permanent benchmark before claiming an optimization result. Source inspection establishes the unnecessary O(N) copy independently of timing noise.

**Acceptance:** retain COW for live aliases, eliminate it for expired aliases, prove destructor/ownership behavior with runtime/core tests, cover nested scopes and every control-flow exit, and remeasure small/large arrays. Handle cleanup in the direct loop execution path as well as AST fallback. Follow the lexical scope contract without changing observable live-alias behavior.

### T11 — P2: Precompute conversion execution and fold safe literal conversions

- [ ] Replace per-evaluation factor parsing and operator resolution with prepared scale/cast plans.
- [ ] Store folded literal conversion results when the operation is context independent.

**Evidence:** `runtime/evaluation.rs:290` converts scale numerator/denominator to strings, parses factors and resolves operators each execution. `cast_base` at line 440 formats a value and parses it back to cast. `Convert`, `DynamicConvert`, and comparison scaling use this path. Binary arithmetic already has `TypedScalePlan` / `execute_scale_plan`, so a reusable mechanism exists. `TypedConversionPlan` stores a scale but not executable scale steps.

`literal_conversion_succeeds` computes an answer during compilation but discards the converted value; `compile_array_element` still emits `Convert`, so known conversions are repeated every time the literal initializer executes. Some context-aware operations are deliberately not folded: preserve that correctness distinction and active configuration behavior.

**Acceptance:** benchmark constrained initialization in loops, dynamic conversion, scaled comparisons and base casts; count allocations/registry lookups where feasible. For fixed known representations, no number formatting/parsing or signature resolution per successful operation. Keep dynamic promotion correct and retain accurate overflow/inexact diagnostics. No speedup claim without before/after measurement.

### T12 — P2: Remove avoidable ownership work from reads and numeric index extraction

- [ ] Add a direct local-slot element access path that borrows the array instead of cloning its owner.
- [ ] Replace bigint index formatting/parsing with checked numeric conversion.
- [ ] Measure specialization of stable array access in loop execution.

**Evidence:** `eval(Variable)` clones its `Value`; `ElementAccess` evaluates the whole array through that path on every read, incrementing/decrementing its Arc even though only one element is needed. `crates/primitives/src/integer/bigint/value.rs:19` calls `to_string().parse::<usize>()` for every bigint index. `runtime/loop_execution.rs:214` and `:311` send array/dynamic nodes to AST fallback.

**Implementation direction:** evaluate an index once with the required ordering, then borrow a known local array and clone/move only what the result needs. Keep a generic path for arbitrary array-producing expressions. Use the existing BigInt library's checked numeric conversion; respect direct dependency conventions if a conversion trait is required. Avoid representation branching inside a loop that can carry a prepared extractor/access plan.

**Acceptance:** benchmark constant/dynamic reads, int8/int64/int128/bigint indexes and read-modify-write separately; preserve side effects, COW, bounds checks and error behavior. Measure whether integrating stable reads/stores into the existing loop plan pays off before expanding that machinery.

### T13 — P2: Bound compiler state copying and dispatch-table growth

- [ ] Restrict/canonicalize dynamic plans where possible and avoid copying all element records at every branch.

**Evidence:** `arrays.rs:461` clones the complete binding-to-element-vector map for snapshots; branch compilation makes additional copies and merges. `compile_block` does not remove dead array records. `dynamic_binary` and `dynamic_comparison` allocate up to `(registered_subtypes + 1)^2` entries per two-dynamic-operand expression, including unrelated registered subtypes. Identical expressions build independent tables; inference also materializes a separate vector of element types.

These are primarily compile-time/IR-memory costs, not evidence that direct indexed dispatch is slow. Dense tables can be an excellent runtime choice; do not replace O(1) selection with runtime HashMap lookup merely to save memory.

**Implementation direction:** clear expired records, snapshot changed bindings/elements or share immutable records, reuse identical immutable dispatch tables, and track reachable subtype sets where sound. Preserve unknown future stores and partial-candidate runtime diagnostics. Do not narrow candidates based on an unsound pre-loop snapshot (T01).

**Acceptance:** compiler-only time and peak memory benchmarks varying array length, branch count, expression count and registered subtype count; unchanged runtime behavior and no regression in dispatch throughput. Large sparse/custom registries should not cause uncontrolled per-expression quadratic growth.

### T14 — P2: Correct and strengthen the test suite

- [ ] Add the missing regressions from T01–T08 and lifetime tests from T10.
- [ ] Fix misleading/over-specific compiler assertions and restore ownership-aligned test placement.

Concrete findings in `crates/compiler/src/compiler/arrays.tests.rs`:

| Existing test | Problem / required change |
|---|---|
| `converts_compatible_elements_to_declared_subtype` (line 136) | Requires a `Convert` node for a known literal. This locks in deferred conversion and conflicts with T11. Check resulting value/contract, allowing a correct folded literal. |
| `drops_element_record_after_dynamic_write` (line 213) | Reads through a dynamic index and only checks `subtype == None`; that happens even without record invalidation. Add a **constant** read after a dynamic write and verify stored-subtype behavior. |
| `widens_adaptive_int_element_beyond_declared_width` (line 282) | Only asserts the output base differs. It cannot detect selecting bigint instead of an available fixed wider type, or any negative/inferred/constrained gap. |
| `rejects_fixed_width_element_overflow` (line 274) | Only covers an overflowing literal; it misses runtime-promoted expressions and subtype-qualified stores. Keep it and add boundary execution tests. |
| `rejects_inexact_literal_element_conversion` (line 424) | Only default int. It misses fixed-width truncation and variables (T05). Its expectation also needs reconciliation with shared scalar conversion semantics. |
| `keeps_declared_element_subtype`, `infers_identical_complete_element_type`, `preserves_constrained_element_type_on_read`, related subtype tests | Several assertions check only `is_some()`. Assert the exact subtype ID and actual converted magnitude so the wrong unit cannot pass. |
| `dispatches_dynamic_element_conversion` / partial candidate tests | `any(Some)` / `any(None)` does not prove the correct subtype slot, scale or result base. Assert concrete valid/invalid candidates and execute representative paths. Do not freeze table dimensions if plan sharing/reachability analysis changes them. |
| `rejects_whole_array_reassignment` (line 294) | Encodes the undocumented exception to mutable-binding rules; resolve T09 before retaining this expected failure. |

**Important distinction:** the 57 `.eckt` tests pass their recorded expectations. Most are useful but incomplete, not tests with demonstrated wrong stdout. The whole-assignment expectation has a documentation conflict; nested/nullable/COW expectations concern incompletely specified features. Do not delete valid regression coverage just because adjacent behavior is broken.

Missing direct unit coverage found in the inspected sibling suites:

- Parser expression tests do not directly cover `ArrayLiteral` or `ElementAccess`; statement tests do not directly cover the new array annotation/indexed-assignment grammar. Add precedence (`-a[0]`, `!a[0]`, binary/logical indexes and pipes), multiline/trailing comma, missing delimiter/element/index, constrained annotations, and invalid assignment-target cases.
- Runtime tests do not directly exercise array read/store/COW behavior; core value tests do not directly exercise `ArrayValue` ownership/mutation. Test unique/shared owners, rejected stores preserving the previous value, alias chains, and scope cleanup, using runtime tests for runtime semantics.
- Integer `value.tests.rs` files do not directly exercise the added `to_index` helpers. Cover zero, representable maxima, negative values, platform-word overflow, and wrong payload types. Keep architecture-dependent limits portable.
- Add regular bool, decimal, float/double and registered extension element cases; current coverage is predominantly integer quantities and a small string sample. Include configuration-sensitive conversion results.

**Layout:** move compiler-owned use cases to `testing/use-cases/compiler/compiler/arrays/` and place parser/runtime-specific cases under their actual owners, following `.agents/docs/eck-lang/use-cases.md`. `language/arrays` is a test-only taxonomy. Use one or two related cases per file. `arrays.tests.rs` also omits the prescribed `use super::*;` import; align its ownership/imports while maintaining valid facade/integration tests where appropriate.

### T15 — P2: Build a meaningful array benchmark matrix

- [ ] Add measured workloads that separate the costs under investigation and validate results before comparing timings.

The single `testing/benchmarks/language/arrays/array_element_access.eckB` uses eight elements, a constant index, 100,000 iterations and a running total. It exercises one useful path but combines reads, writes, integer addition, loop overhead and accumulation. It does not cover dynamic indexes, aliases, allocation, conversion, wide elements or dispatch growth.

`testing/BENCHMARKS.md` specifies that the runner builds **HEAD in a detached worktree**. It does not measure current uncommitted array changes. Do not report a HEAD checkpoint as a baseline for the working tree audited here. No commits were created by this audit.

Required workloads, with operations/inputs/units stated explicitly:

| Workload | Vary / measure |
|---|---|
| Stable reads | constant vs dynamic index, small vs cache-exceeding arrays; ns/read |
| Writes | unique owner, live alias, expired alias; ns/store, copies and allocation bytes |
| Creation | constant vs evaluated elements, repeated block creation; time/array and bytes |
| Subtypes | constrained vs mixed; read-only arithmetic, comparison, conversion; time/operation |
| Widths | int8/int64/int128/bigint and string; index conversion and payload costs separately |
| Compilation | subtype count, branch count, literal size, repeated dynamic expressions; compile time and peak memory |

Use an in-process compile-once/runtime benchmark for runtime claims, plus end-to-end CLI checks for user-visible latency. Keep constant folding from accidentally removing the workload; validate the final result. Add the T10 experiment as a permanent reproducible benchmark and repeat after cleanup. Record medians, samples, build profile, environment and relevant commit/working-tree state.

### T16 — P3: Evaluate storage specialization after fixing contracts

- [ ] Profile whether fixed homogeneous arrays warrant packed storage and batch operations.

`ArrayValue` is a contiguous `Vec<Value>`, which is already preferable to a linked/per-element-container representation. Ordinary numeric `Value` payloads are inline; **it would be incorrect to claim every numeric element allocates**. However each element retains general type/subtype/payload metadata, and strings/bigints may have separately shared payloads. A fixed-width homogeneous array could have a substantially denser representation.

This is a profiling/design task, not a proven required rewrite. Compare cache behavior and throughput before introducing storage variants. Keep per-element subtype information for unconstrained arrays and adaptive representation freedom where required. Specialize only justified paths and retain extension support. A plain vector plus borrowed reads may deliver much of the gain at lower complexity.

## Recommended implementation order and completion gate

1. T01–T04 and T08: restore sound flow and enforce real stored contracts.
2. T05–T07: consistent exact conversion, widening and indexing, coordinated with shared scalar code.
3. T09: resolve assignment/copy and missing specification decisions; update affected tests from the decided semantics.
4. T10: restore lexical owner lifetime and remove avoidable full copies.
5. T14/T15: land regressions and benchmarks alongside each relevant fix, not as a final optional sweep.
6. T11–T13: optimize verified hot paths and bound compiler work; evaluate T16 only with measurements.

Before handoff is complete: run focused array cases, `cargo test --workspace` and `cargo test-all`; check debug/release consistency on new regressions; compare benchmarks against a baseline that actually contains array support. Report what was measured, unresolved specification questions and any intentionally deferred items. Do not mark the array implementation complete solely because the pre-existing suites remain green.
