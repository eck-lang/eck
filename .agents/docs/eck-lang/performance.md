# Performance

Raw execution speed is a primary design goal of ECK. The time and CPU cost of
running an ECK program takes precedence over every implementation concern that
does not affect observable behavior. A feature is not finished when it merely
works; it is finished when it works and runs as fast as a correct
implementation can run.

This is a language-level commitment, not an optional optimization pass.
"Premature optimization" does not apply to a deliberate choice of algorithm or
representation: choose the fastest correct design up front instead of building
a slow abstraction and hoping to optimize it later.

## Decision rule

Correctness is a prerequisite: a faster implementation is only valid when it
produces the same observable result as the correct one, including diagnostics,
formatting, precision, scale, and ordering. Among correct implementations,
always choose the one with the lowest raw cost on the hot path.

A hot path is any code that executes per statement, per value, per row, per
batch element, or per arithmetic, comparison, or function operation. The
following are not acceptable reasons to slow a hot path down:

- a shorter or more readable implementation when a materially faster one
  exists;
- symmetry with an unrelated, cold code path;
- avoiding a small amount of additional code or compile-time work at the cost
  of runtime work;
- deferring the fast path to a hypothetical future rewrite.

## Resolve work at compile time

Move work out of execution whenever the result is known when the program is
compiled. The IR already resolves names, types, operators, comparisons, and
functions to identifiers; extend that principle so the runtime never repeats
work the compiler can do once:

- never resolve a name, operator, comparison, function, subtype suffix, or
  configuration setting at runtime when the compiler can resolve it;
- precompute literal conversions, operand scales, and promotion decisions at
  compile time;
- keep resolved identifiers and dispatch tables in the IR rather than
  reconstructing them per evaluation;
- reject an implementation that re-parses, re-matches on type, or re-looks-up
  a value that was already resolved earlier in the pipeline.

## Avoid per-operation overhead

Per-value and per-operation overhead dominates the cost of an interpreted
runtime. Do not pay it:

- do not allocate on the hot path when a reusable buffer, arena, or pool can
  hold the result;
- do not box, clone, or reference-count a value that can be handled inline;
- do not use dynamic dispatch or a branch-per-variant when a statically
  dispatched or monomorphic path is available;
- do not branch on representation inside a loop that already knows the
  representation;
- hoist invariant computation and conversions out of loops, and avoid
  recomputing the same value more than once.

Prefer contiguous, column-oriented storage and process values in tight vector
loops rather than through per-element indirection. In the frame and data path,
operate on whole columns and batches, not one row at a time, and apply the same
rules to `ColumnExpression` and aggregation state so partial states stay cheap
to merge.

## Choose data structures deliberately

Pick algorithms and containers for their asymptotic behavior and their
constant factors, not for familiarity. Prefer a vector or a slice over a map
when the key space is small or dense, reserve capacity when the final size is
known, and avoid rebuilding a collection that can be updated in place. When a
representation forces a slower structure to satisfy an unrelated abstraction,
change the abstraction instead of accepting the cost.

## Measurement

Never describe a change as faster without measuring it. Performance claims must
be backed by a benchmark under `benchmarks/` or by an existing benchmark that
the change improves.

- Add or extend a benchmark when a change touches a hot path.
- State the operations, inputs, and units a benchmark measures so results are
  comparable across runs.
- Treat a regression on an existing benchmark as a bug unless the change has
  an explicit, documented reason and the trade-off is accepted.
- Optimize what the measurement shows: profile before rewriting, and verify
  the speedup after.

## Accepted trade-offs

When the same observable behavior can be preserved, runtime speed may be bought
with any of the following, and the choice must be deliberate and documented in
the code or commit message:

- additional compile-time work;
- larger generated code or data;
- higher memory use for cached or precomputed state;
- more implementation code and specialized cases.

These costs are secondary. Compile-time and memory use must not grow without
bound, but a deliberate, measured runtime speedup is worth a slower compile, a
larger binary, or more code under this commitment.
