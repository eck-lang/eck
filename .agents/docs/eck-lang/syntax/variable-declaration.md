# Variable declaration syntax

Declare variables with `let` or `const`, an optional type annotation, and a
required initializer:

```eck
let mutable_value: int = 10
const fixed_value: string = "hello"
let inferred_value = true
let optional_value: int? = null
```

- `let` creates a mutable binding; `const` creates an immutable binding.
- An explicit type annotation creates a static assignment contract.
- An unannotated mutable `let` has a dynamic assignment contract. Its initializer
  supplies current compiler knowledge, not a restriction on later assignments.
- An unannotated `const` keeps the initializer's type because it cannot be
  reassigned.
- A type annotation may be any structural type expression, including aliases,
  unions, recursive arrays, and nullable postfix forms; see
  [type expressions](type-expressions.md).
- The initializer must match the declared type and cannot produce no value.
- A binding is visible from its declaration to the end of its lexical scope.
- Redeclaration in the same scope is invalid; shadowing in a child scope is
  allowed.
- Assignment without a prior declaration never creates a variable.

```eck
let value = 10
value = "hello"
value = true
value = null
value = [1, 2, 3]
```

Every assignment stores the concrete value produced by its expression. Dynamic
bindings do not parse, format, or coerce values; a numeric-looking string stays
a string. Compiler flow knowledge may specialize operations while the current
type is known, but that knowledge never becomes a binding contract.

Do not declare variables without `let` or `const`:

```eck
value: int = 10 // Invalid.
```
