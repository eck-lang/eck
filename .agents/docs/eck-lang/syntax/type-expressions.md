# Type expressions

Type annotations use structural type expressions. A type alias expands to its
underlying expression; it does not introduce a nominal runtime type:

```eck
type Value = string | int
type Number = int | decimal
type Row = Number[]
type Matrix = Row[]
```

Aliases may refer to aliases declared later in the same program. Recursive
aliases are not supported; a cycle is a compile-time error.

## Unions

Use one `|` between alternatives:

```eck
let value: string | int = 10
let text_or_number: Value = "hello"
```

Unions are canonicalized during compilation: nested unions are flattened,
duplicate concrete identities are removed, and member order is deterministic.
Aliases are expanded before this normalization. `int` and `int64` refer to the
same runtime identity, so `int | int64` has one scalar member while retaining
the adaptive `int` representation policy.

The single `|` token is distinct from the existing `||` logical operator.

## Postfix precedence

Array and nullable postfix operators can be repeated and compose in source
order:

```eck
int[][]
int[][][]
int?[]
int[]?
int?[]?
```

Parentheses determine whether a union is inside an array or outside it:

```eck
int[] | string[]     // one homogeneous int[] or one homogeneous string[]
(int | string)[]     // one array whose elements may be int or string
(int | string)[][]   // an array of arrays of those union elements
```

These shapes remain distinct in the static type system. Mutable arrays are
invariant, so `int[]` cannot be assigned to `(int | string)[]`; otherwise a
string could be inserted through the wider binding. An `int[]` can be assigned
to `int[] | string[]` because it directly satisfies one union member.

## Nullable lowering

`T?` is lowered to `T | null` during semantic resolution. The position of the
postfix operator matters:

```eck
int[]?       // an array value or null
int?[]       // a non-null array whose elements may be null
int?[]?      // an array of nullable elements, or null
```

Nullable values must be narrowed with the existing null comparison before a
scalar operation or before entering a non-nullable array slot. Nullable arrays
may be tested for null and then indexed or mutated in the non-null branch.
Flow-sensitive union narrowing beyond the existing null proof, `is`, `match`,
and runtime reflection remain out of scope.

## Runtime boundary

Aliases and unions are compile-time constructs. Runtime `Value` identities stay
concrete: a value is one scalar identity or one concrete recursive array
identity, never a generic union wrapper. Homogeneous arrays therefore retain
their existing specialized contracts, while union-element arrays use the
existing value payload as the minimal heterogeneous fallback.
