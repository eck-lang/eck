# Array declaration syntax

Declare arrays with square brackets and use `[]` after a type annotation to
constrain their element type:

```eck
let numbers: int[] = [1, 2, 3]
let fixed_numbers: int64[] = [1, 2, 3]
let names: string[] = ["alice", "bob"]
let inferred = [1, 2, 3]
```

* An array contains values with the same declared base type.
* When the array type annotation is omitted, its element type is inferred from
  the initializer.
* `int[]` uses the normal adaptive `int` semantics. Integer values may widen as
  required without changing the declared array type.
* Explicit-width integer arrays such as `int64[]` are fixed to that integer
  width. A value that cannot be represented by the declared width is invalid.
* An empty array requires an explicit element type because its type cannot be
  inferred from its contents.

The physical storage a future `int[]` may use is a design question rather than
part of the syntax contract; the intended direction is recorded in
[the adaptive integer array design note](../adaptive-integer-arrays.md).
Fixed-width arrays keep their strict representation contract either way.

```eck
let values: int[] = []
let inferred = [] // Invalid: element type cannot be inferred.
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

A value whose subtype cannot be converted to the declared subtype is invalid:

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

Unlike `int`, an explicit-width integer type does not widen when a value exceeds
its representable range:

```eck
let flexible: int[] = [1]
let fixed: int64[] = [1]

flexible[0] = 999999999999999999999999999999
fixed[0] = 999999999999999999999999999999 // Invalid.
```

## Type inference

An array literal with elements of the same complete type may infer that type:

```eck
let sizes = [10mm, 20mm, 30mm]
```

The inferred type is:

```eck
int<mm>[]
```

When the elements share a base type but have different subtypes, the common base
type is inferred and each element keeps its subtype:

```eck
let sizes = [10mm, 2cm, 3dm]
```

The inferred array type is:

```eck
int[]
```

Elements that do not share a compatible base type cannot form an inferred
array:

```eck
let values = [
    10mm,
    "hello"
] // Invalid.
```

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
