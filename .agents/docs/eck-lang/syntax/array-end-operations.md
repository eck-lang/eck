# Array end operations

A mutable array supports built-in operations that add or remove one value at
either end. They use the same `->` method syntax as every other ECK operation:

```eck
let values: int[] = []

values->push(10)
values->pop()

values->unshift(5)
values->shift()
```

| Method | Alias | Effect | Result |
| --- | --- | --- | --- |
| `push(value)` | `append(value)` | adds `value` after the last element | no value |
| `unshift(value)` | `prepend(value)` | adds `value` before the first element | `T?` |
| `pop()` | — | removes the last element | `T?` |
| `shift()` | — | removes the first element | `T?` |

* `append` is the `push` operation and `prepend` is the `unshift`
  operation. An alias names the same operation, with one set of rules and one
  implementation.
* The receiver must be a mutable array binding written directly by name. A
  `const` binding, a temporary such as an array literal, and any other
  receiver are invalid.
* An adding method requires exactly one value; a removing method takes no
  argument.

These current array operations are compiler/runtime intrinsics. They are not
standard-library functions or registry-native methods, so this syntax does not
imply a generic array functions directory or a separately registered function
family.

## Element order

```eck
let a: int[] = []

a->push(10)
a->push(20)
a->unshift(5)

print(a)
```

prints:

```text
[5, 10, 20]
```

Removing at either end then leaves `[10]`:

```eck
let first = a->shift() // 5
let last = a->pop()    // 20
```

## Empty arrays

A removal from an empty array has no element to produce, so both removing
methods produce null:

```eck
let value: int? = a->pop()
let first: int? = a->shift()
```

For a non-empty `T[]`:

```text
pop()   -> T?
shift() -> T?
```

The complete element type is preserved. An element stored with a subtype is
returned with that subtype, so removing from an array that holds `2cm`
produces `2cm`.

Because the result may be null, it must be narrowed before it is used as a
scalar:

```eck
let sizes: int[] = [10mm, 2cm]

let width = sizes->pop()
if (width != null) {
    print(width->to(mm))
}
```

A declaration that omits its annotation takes the nullability of its
initializer, so the binding above is nullable. Declaring it non-nullable is
invalid, because null would have no representation there:

```eck
let width: int = sizes->pop() // Invalid.
```

## Element contract

An inserted value satisfies the destination array's element contract exactly as
a literal element and an indexed assignment do. Insertion has no conversion
path, coercion, or overflow rule of its own.

```eck
let values: int[] = []
values->push(10)
values->append(20)
```

An unannotated array has a dynamic element contract, so insertion stores the
concrete `Value` directly without validation or conversion:

```eck
let values = []
values->push(10)
values->push("10") // remains a string
```

A subtype-constrained array converts a compatible value before storing it, so
`sizes->push(2cm)` stores `20mm` in `int<mm>[]`.

A fixed-width element keeps its representation, so inserting a value the width
cannot hold is invalid. The same rejection applies when the value only needed a
wider representation while it was evaluated:

```eck
let values: int8[] = []
values->push(300) // Invalid: int8 cannot represent 300.
```

An unconstrained array keeps whatever subtype the stored value has, exactly as
it does for a literal element. A later read dispatches on the subtype the
element actually carries, because an insertion and a removal both change which
values occupy the array's positions.

## Implementation notes

The following details describe the current implementation rather than adding
observable syntax requirements.

### Cost

Both ends of an array are constant-time:

```text
push, append      O(1) amortized
pop               O(1)
unshift, prepend  O(1) amortized
shift             O(1)
indexing          O(1)
```

The current array keeps one contiguous live payload with reusable front and back
spare capacity. The amortized cost comes from occasionally growing and
recentering the allocation; `unshift` and `shift` do not move every remaining
element on each call. Copy-on-write uses the existing `Value` ownership model,
and lexical slot cleanup releases expired owners while preserving live aliases.
