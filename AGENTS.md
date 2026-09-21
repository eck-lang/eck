# Project instructions

LLM-specific project documentation lives in `.agents/docs/`.

Read the relevant files before making changes:

* `.agents/docs/repository.md` — repository-wide conventions including language
* `.agents/docs/code-style.md` — naming and code-style conventions
* `.agents/docs/commits.md` — commit message and commit workflow conventions
* `.agents/docs/eck-lang/crate-structure.md` — module ownership and operation layout
* `.agents/docs/eck-lang/adaptive-integer-arrays.md` — current adaptive `int[]` storage contract and deferred vectorized execution design
* `.agents/docs/eck-lang/use-cases.md` — language use-case test conventions
* `.agents/docs/eck-lang/performance.md` — raw execution speed as the primary implementation goal
* `.agents/docs/eck-lang/syntax/array-declaration.md` — array literals, element constraints, indexing, mutation, and rendering
* `.agents/docs/eck-lang/syntax/type-expressions.md` — structural aliases, unions, recursive arrays, precedence, and nullable lowering
* `.agents/docs/eck-lang/syntax/array-end-operations.md` — adding and removing values at either end of a mutable array, including the aliases and the empty-array contract
* `.agents/docs/eck-lang/syntax/conditionals.md` — current `if`, `else if`, and `else` syntax, including mandatory condition parentheses
* `.agents/docs/eck-lang/syntax/for.md` — current integer range `for` loop syntax and behavior
* `.agents/docs/eck-lang/syntax/variable-declaration.md` — variable declaration syntax, mutability, type inference, and nullability
* `.agents/docs/eck-lang/syntax/while.md` — current `while` loop syntax, condition requirements, and control flow
* `.agents/docs/eck-lang/variable-scope.md` — binding, scope, shadowing, and control-flow scope semantics
* `.agents/docs/tests.md` — testing conventions, structure, and requirements

When a task modifies files under `.agents/docs/`, review this index as part of
that task and update it if any path or description is no longer accurate. Do
not perform this synchronization check for tasks that do not modify
`.agents/docs/`.

The documentation in `.agents/docs/` is authoritative.

Follow the relevant documentation exactly and do not invent alternative project conventions when they are already defined there.

When implementing or changing a feature that affects language syntax, read the
relevant documentation under `.agents/docs/eck-lang/syntax/` first. The
implementation and its tests must conform to the documented syntax, even when
the existing implementation accepts a conflicting form.
