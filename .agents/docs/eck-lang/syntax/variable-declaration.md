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
- When the type annotation is omitted, the initializer determines the type.
- A nullable declaration uses `?` after an explicit type name.
- The initializer must match the declared type and cannot produce no value.
- A binding is visible from its declaration to the end of its lexical scope.
- Redeclaration in the same scope is invalid; shadowing in a child scope is
  allowed.
- Assignment without a prior declaration never creates a variable.

Do not declare variables without `let` or `const`:

```eck
value: int = 10 // Invalid.
```
