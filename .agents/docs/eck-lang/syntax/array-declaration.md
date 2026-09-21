# Array declaration syntax

Arrays are built-in containers. They are neither primitive scalar values nor
standard-library values. Declare them with square brackets and use `[]` after a
type annotation to constrain their element type:

```eck
let numbers: int[] = [1, 2, 3]
let fixed_numbers: int64[] = [1, 2, 3]
let names: string[] = ["alice", "bob"]
let dynamic = [1, 2, 3]
```

* An array element contract may be scalar, nullable, a union, or another
  recursive array. `int[] | string[]` is a union of homogeneous arrays, while
  `(int | string)[]` is one array with a union element contract.
* When the array type annotation is omitted in a mutable dynamic context, the
  array has a dynamic element contract. Its elements keep their concrete types,
  and later insertions or indexed writes may use unrelated types.
* `int[]` uses adaptive signed integer semantics with the widening progression
  `int8 -> int16 -> int32 -> int64 -> int128 -> bigint`. Integer values may
  widen as required without changing the declared array type.
* Explicit-width integer arrays such as `int64[]` are fixed to that integer
  width. A value that cannot be represented exactly by the declared width is
  invalid.
* An empty unannotated array is valid and starts with a dynamic element contract.

The physical storage is not part of the syntax contract. Current storage and
future adaptive-layout proposals are recorded in
[the adaptive integer array design note](../adaptive-integer-arrays.md).

```eck
let values: int[] = []
let dynamic = []
dynamic->push(10)
dynamic->push("hello")
```

## Element subtypes

An array declaration may omit an element subtype:

```eck
let sizes: int[] = [
    10mm,
    2cm,
    3dm
]
```

The array constrains the base type to `int`, while each element preserves its
own subtype:

```eck
let first = sizes[0]  // int<mm>
let second = sizes[1] // int<cm>
let third = sizes[2]  // int<dm>
```

Values with different subtypes may therefore coexist in the same array when
their base type satisfies the array declaration:

```eck
let values: int[] = [
    10mm,
    5kg,
    2s
]
```

The container does not change the semantic type of an element. Reading an
element from an array produces the same type and subtype the value would have
outside the array.

## Constrained element subtypes

A subtype may be specified as part of the array element type:

```eck
let sizes: int<mm>[] = [
    10mm,
    20mm,
    30mm
]
```

This constrains both the base type and the subtype of the array elements.

Compatible values expressed with another unit may be converted to the declared
subtype before they are stored:

```eck
let sizes: int<mm>[] = [
    10mm,
    2cm,
    1dm
]
```

The resulting elements are treated as:

```eck
10mm
20mm
100mm
```

A value whose subtype cannot be converted exactly to the declared subtype is
invalid:

```eck
let sizes: int<mm>[] = [
    10mm,
    5kg // Invalid.
]
```

## Fixed-width integers and subtypes

Explicit-width integer types follow the same subtype rules:

```eck
let distances: int64[] = [
    10mm,
    2cm
]

let normalized: int64<mm>[] = [
    10mm,
    2cm
]
```

`int64[]` constrains the integer representation while leaving the subtype
unconstrained.

`int64<mm>[]` constrains both the integer representation and the subtype.

For fixed-width integer arrays, a unit conversion must prove exact
representability before division or truncation. A fractional result is not
silently truncated to fit the destination width.

Unlike `int`, an explicit-width integer type does not widen when a value exceeds
its representable range:

```eck
let flexible: int[] = [1]
let fixed: int64[] = [1]

flexible[0] = 999999999999999999999999999999
fixed[0] = 999999999999999999999999999999 // Invalid.
```

For a non-nullable element contract, nullable expressions cannot cross into an
array element slot; narrow them first. A nullable element contract such as
`int?[]` explicitly stores either an integer or the concrete `null` value.
Likewise, `int[]?` is a nullable array binding and can be indexed after the
existing null comparison proves that the array itself is present. Mutable
arrays are invariant: an `int[]` is not assignable to `(int | string)[]`.

The complete precedence and alias rules are documented in
[type expressions](type-expressions.md).

## Recursive arrays

Array postfix syntax is recursive and may be repeated:

```eck
let matrix: int[][] = [[1, 2], [3, 4]]
let cube: int[][][] = [[[1]]]
let mixed: (int | string)[][] = [[1, "one"]]
```

Nested array values keep a concrete recursive `ArrayType` at runtime. The
runtime does not carry a generic union identity or walk static union members.

## Dynamic array literals

An unannotated mutable array literal does not infer a lasting element contract:

```eck
let values = [1, 2, 3]
values->push("hello")
values->push(true)
values->push(null)
```

Each element retains its concrete runtime identity. Strings that look numeric
are never parsed implicitly:

```eck
let values = [1, "2", 3]
```

Nested arrays are ordinary values and keep their own contracts. A typed array
moved into a dynamic binding also keeps its static element contract.

An explicit destination context constrains a literal directly:

```eck
let values: int[] = [1, 2, 3]
```

When a dynamic array crosses into a typed array binding, the runtime validates
the complete value once when static proof is unavailable. A successful crossing
creates a typed array value, so later accesses do not repeat the boundary check.

## Element access

Access an array element with its zero-based index:

```eck
let sizes: int[] = [10mm, 2cm]

let first = sizes[0]
let second = sizes[1]
```

Element access preserves the complete semantic type of the stored value:

```eck
// first: int<mm>
// second: int<cm>
```

Operations on an extracted element use the normal rules for that type and
subtype. Arrays do not introduce separate arithmetic or conversion semantics
for their elements.

## Rendering

An array is rendered as a bracketed, comma-separated list of its elements, in
their stored order:

```eck
let sizes: int[] = [10mm, 2cm]
print(sizes) // [10mm, 2cm]

let empty: int[] = []
print(empty) // []
```

Every element keeps the complete identity it was stored with: its actual base
representation and subtype. Its subtype suffix and active configuration apply
exactly as they do outside the array.

## End operations

Adding and removing a value at either end of a mutable array is documented in
[array end operations](array-end-operations.md).

## Implementation boundary

The following details are implementation notes, not additional source
semantics. `eck-lang::containers::array` owns the canonical `ArrayType`,
`ArrayElementContract`, payload, storage, compiler rules, runtime boundary, and
formatter. `semantic` only re-exports the vocabulary needed by shared value and
type infrastructure. Finite element dispatch preserves complete identities and
uses dense compiler-planned candidates rather than textual lookup on hot paths.
The array-owned flow state also preserves a conservative finite whole-array
domain when exact slot positions are lost. A genuinely open scalar operation
delegates through runtime identity to the Registry and caches the prepared plan
at that operation site.
