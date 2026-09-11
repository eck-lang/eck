# Variable scope and binding semantics

This document defines the intended variable scope and binding semantics for
ECK. It covers how bindings are declared, resolved, shadowed, and released,
and how control-flow constructs introduce scopes. Target semantics may not all
be implemented yet; the existing repository is authoritative for what already
works, and conflicts must be resolved explicitly before changing semantics.

The goal is a statically analyzable, ETL-focused model with explicit bindings
and enough compile-time information to support future purity analysis and
automatic parallelization.

## Lexical block scope

ECK uses lexical block scoping. Every `{ ... }` block introduces a child scope.
A binding is visible from the point where it is declared until the end of its
declaring scope, and inside descendant scopes unless shadowed.

```eck
let x: int = 10

if (x > 5) {
    let y: int = 20

    print(x) // valid
    print(y) // valid
}

print(x) // valid
print(y) // compile-time error
```

Name resolution searches the current scope first, then parent scopes
recursively, and otherwise emits an unknown-binding diagnostic:

```text
resolve(name, scope):
    if scope contains name:
        return binding

    if scope.parent exists:
        return resolve(name, scope.parent)

    error UnknownBinding
```

Adapt this to the existing compiler architecture rather than implementing the
pseudocode literally when an equivalent or better mechanism already exists.
Where the grammar allows standalone `{}` block statements, treat the block
itself as the scope-generating primitive instead of duplicating scope logic
inside every control-flow construct.

## Binding declarations

The intended declarations are:

```eck
let x: int = 10
const y: int = 20
```

- `let` declares a mutable binding.
- `const` declares an immutable binding.

There is no implicit declaration through assignment. Assigning to a name that
has never been declared is a compile-time error and must never create a
binding:

```eck
x = 10 // error if `x` was never declared
```

A `let` binding may be reassigned, but its declared or inferred type stays
stable:

```eck
let value: int = 10

value = 20    // valid
value = "20"  // type error
```

A `const` binding cannot be reassigned.

## Redeclaration and shadowing

Redeclaration in the same lexical scope is invalid:

```eck
let x: int = 10
let x: int = 20 // compile-time diagnostic
```

Shadowing in a nested scope is allowed:

```eck
let x: int = 10

if (true) {
    let x: string = "hello"
    print(x)
}

print(x)
```

The two bindings are distinct symbols and may have different types. The binding
model must make symbol identity explicit so later compiler analyses can reason
about symbols rather than only textual names.

## Control-flow scopes

Each block owned by `if`, `else`, `while`, or `for` is a lexical child scope.
Branch-local declarations are not merged into the parent scope:

```eck
if (condition) {
    let x: int = 10
} else {
    let x: int = 20
}

print(x) // invalid
```

For loops, loop-local bindings such as the iterator must not escape:

```eck
for (item in items) {
    print(item)
}

print(item) // invalid
```

## Foundation for static analysis

Scope and symbol resolution must support future static analyses, notably purity
analysis and automatic parallelization. The implementation must be able to
distinguish an outer-scope read from an outer-scope write:

```eck
let offset: int = 10

for (row in dataframe) {
    let result = row.value * 2 + offset // reads `offset` from a parent scope
}
```

```eck
let total: int = 0

for (row in dataframe) {
    total = total + row.value // writes `total` in a parent scope
}
```

Identifiers must not be resolved only dynamically by textual name. Prefer a
clean symbol/binding model that future compiler passes can annotate with
binding identity, declaration scope, mutability, declared type,
effective/narrowed type, read/write use, and capture from a parent scope. The
full purity and autoparallelization system is out of scope; only the sound
foundation is in scope.

## Diagnostics

Diagnostics use the existing ECK diagnostic framework and formatting. Scope and
binding errors should clearly explain cases such as:

- unknown binding;
- binding already declared in this scope;
- assignment to an immutable binding.

## Required tests

The following behavior must be covered:

- nested lookup and valid use of an outer binding;
- use of a block-local binding outside its block (error);
- same-scope redeclaration error;
- nested-scope shadowing, including shadowing with a different type;
- assignment to an undeclared name;
- assignment to `const`;
- reassignment of `let`;
- type-invalid reassignment;
- branch-local variables not escaping;
- loop-local iterator not escaping.

## Non-goals

This work must not introduce implicit declaration through assignment, implicit
scope merging across branches, or dynamic name resolution.
