# Desired: adaptive parallel profitability cost learning

Status: future design only. The current implementation uses deterministic
static relative work units and never times regions or learns runtime costs.

## Objective and safety boundary

After the same compiled execution region has run enough times, observed cost
could refine its static cold-start estimate for subsequent invocations.
Learning would affect only the sequential-versus-parallel profitability choice.
Resolved purity, dependencies, callback identity, configuration guards, and
worker availability must remain authoritative; no learned estimate may make
an unsafe region parallel.

The intended progression is static prediction, initial executions, sparse
observations, a stable moving estimate, then a gradual blend for future choices.
Do not profile individual IR nodes or individual iterations.

## Stable execution-region identity

Associate observations with a stable internal compiled-plan/region identifier.
Source spans alone are insufficient: edits, imported source, registry callback
revisions, and compiler changes can give different plans the same span.
Invalidate learned data when the execution plan or relevant runtime behavior
changes. Distinguish different native implementations and value configurations.
Any future persistence key needs a versioned plan fingerprint and cost-model
version; the current span-indexed safety table is not such a persistent key.

## Warm-up and moving estimate

Keep static estimates until several valid observations exist. The warm-up count
is a later design decision, chosen through benchmarks rather than one sample.
Exclude pool startup, compilation, and unrelated initialization when possible.
Track iteration counts and execution strategy with samples so observations of
different trip counts are comparable. Setup, per-iteration execution, and
ordered-write replay are distinct sources of elapsed time; their treatment
requires a bounded model rather than a full symbolic cost engine.

An exponential moving average is a candidate: it needs constant storage and
cheap updates instead of retaining every sample. Require a minimum sample size,
clip extreme observations or reject clear outliers, and bound adjustment speed
so one scheduler pause or temporarily loaded host cannot poison future choices.
Failures, cancellation, very short runs, and speculative worker execution need
explicit sample-validity rules; incomplete regions should not look cheap.

## Blending and units

A possible effective estimate is:

```text
effective_cost = static_weight * static_cost
               + observed_weight * learned_cost
```

Start with observed weight zero and increase it only after enough stable
samples. Bound the learned correction and retain the static estimate as the
fallback after invalidation or insufficient evidence. Wall-clock measurements
are not relative work units: define and benchmark a principled conversion or
a separate bounded profitability comparison before using this formula.
Do not silently compare nanoseconds against the current user work threshold.
Consider hysteresis to avoid repeatedly switching near a crossover.

## Machine locality and overhead

Measurements belong to the machine and runtime that collected them. CPU mix,
worker budget, system load, allocator, native input sizes, array replay and
runtime build can change profitability. Do not transfer observations between
machines without an explicit compatible identity and fresh validation.

Sampling must be extremely cheap and infrequent: at most a small number of
clock reads around a whole selected invocation, no per-node timers, no
per-iteration locks or allocations, and bounded plan-local state. Concurrent
invocations need a cheap safe update scheme. Measure the sampling overhead
against both tiny sequential regions and large parallel regions before adopting
it; disable sampling when it costs more than the optimization.

## Optional persistence

Begin by evaluating process-local state. Persistence is an open future decision,
not a requirement. If later justified, discuss cache size/eviction, model and
build versions, machine and worker identities, invalidation, stale values,
privacy, concurrent writers, and explicit reset/disable controls. Avoid creating
a historical-performance database without demonstrated value.

## Required future validation

Compare forced sequential, forced parallel, static-only, and adaptive choices
on the same compiled programs. Include workloads where a fixed static class is
misleading: variable-length string/regex work, ordered indexed writes, changing
trip counts, and changing worker budgets. Report warm-up behavior, steady-state
improvement, overhead, bounded memory, outliers, and host-load sensitivity.

Preserve exact semantic equality and deterministic failure prefixes in every
mode. Prove safety still wins, static fallback works, plan identities cannot
reuse stale samples, and independent invocations cannot corrupt learned state.
Show that adaptive decisions measurably improve over static-only choices on
these workloads without regressing trivial loops. Timing remains evidence of
profitability only; existing thread instrumentation proves real parallel use.
