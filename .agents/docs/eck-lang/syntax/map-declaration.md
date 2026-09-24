# Map declaration syntax

Maps are built-in mutable containers. A map literal uses braces with a colon
between each expression key and value:

```eck
let scores = {"alice": 10, "bob": 20}
let selected = {1: "one", 2 + 1: "three"}
```

Both keys and values are expressions. Keys are evaluated before their values,
and entries are evaluated in source order. An empty map is written `{}`.

## Dynamic contract

The current map syntax creates a dynamic map contract. The literal does not
infer a lasting key or value type, so later indexed assignments may store
different scalar values:

```eck
let values = {"count": 1}
values["label"] = "ready"
values["enabled"] = true
```

The generic `map<K, V>` type expression is deferred. Do not use it in current
source programs; the current language has one dynamic map shape.

## Key identities

Every key must be a hashable scalar value. Map key matching uses the complete
runtime key identity, including the scalar base type and any subtype. Numeric
values with different identities are therefore different keys even when their
formatted values look alike. Keys are not converted to strings and containers
are never used as keys.

`null` is not a valid key. Arrays and maps are not valid keys. Floating-point
NaN is rejected because it does not provide a stable key identity. Other
finite scalar values, including strings, booleans, integers, and finite
floating-point values, may be keys when their scalar type is registered as
hashable.

## Lookup

Use square brackets with a key expression:

```eck
let values = {"answer": 42}
print(values["answer"])
print(values["missing"])
```

Lookup returns the stored value when the complete key identity is present.
Lookup of a missing key returns the concrete `null` value; it does not raise an
index error.

## Assignment and replacement

Indexed assignment inserts a key when it is absent and replaces the value when
the complete key identity is already present:

```eck
let values = {"answer": 41}
values["answer"] = 42
values["new"] = "value"
```

The binding must be mutable. A `const` map can be read but cannot receive an
indexed assignment.

When a key is replaced, its original position is retained. A newly inserted
key is appended after the existing entries.

## Rendering

Maps render as a brace-delimited, comma-separated list of `key: value` pairs in
insertion order. Map string keys and string values use quoted, escaped source
syntax. Non-string values keep their normal formatting:

```eck
let values = {"first": 1, "second": 2}
print(values)
// {
//     "first": 1,
//     "second": 2
// }
```

Replacing an existing key changes only its value, so the rendered order stays
stable. Empty maps render as `{}`. Keys and values keep their normal scalar
formatting.

## Implementation boundary

Map storage and lookup are runtime container operations, while map literals,
key validation, and indexed assignment are compiled language operations. The
map value retains its dynamic container identity; it is not a scalar operand
and cannot be used in arithmetic or as a map key.
