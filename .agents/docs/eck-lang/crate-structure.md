# Crate and Primitive Module Structure

Built-in primitives share the `eck-primitives` crate and each live in a
dedicated `src/<primitive>/` module. Standalone extensions remain separate
crates. Primitive modules and extension crates use the same internal separation
of concerns even though their package boundaries differ.

Use `eck-primitives/src/decimal` as the reference for a primitive that defines
a value type and multiple operation implementations. Include only the modules
required by the primitive or extension's capabilities.

## Cargo manifest

Every crate must have a `Cargo.toml` that:

- names the package `eck-<crate-name>`;
- inherits the workspace version, edition, and license;
- declares every direct runtime dependency in `[dependencies]`;
- declares dependencies used only by tests in `[dev-dependencies]`;
- inherits third-party dependencies from the workspace when available.

Internal ECK-Lang dependencies must specify both their package name and their
relative path. Do not rely on transitive dependencies.

## Source structure

`lib.rs` is the crate façade. In `eck-primitives`, it declares the primitive
modules and re-exports their extension types. Each primitive's `mod.rs`
implements `language_core::Extension` and coordinates that primitive's
registration. A standalone extension crate performs those responsibilities in
its own `lib.rs`.

A primitive module or extension crate that defines a runtime value should
separate the relevant concerns as shown by the decimal module:

- `literal.rs` parses source literals;
- `formatting.rs` formats runtime values;
- `value.rs` accesses and validates the runtime payload and owns reusable
  conversions into that representation;
- `operations/` implements and registers arithmetic operators;
- `comparisons/` implements and registers comparison relations.

Omit modules for capabilities the crate does not support. Keep implementation
modules private or `pub(crate)` unless they intentionally belong to the public
API.

## Arithmetic operations

Primitive modules and extension crates that implement or register language
operations must have an `operations/` directory with a `mod.rs` that
coordinates the operation modules and their registration.

When an operation has one implementation module, place it directly in
`operations/` and name it `<operation>_<type>.rs`. Integer addition therefore
uses:

```text
operations/
├── mod.rs
└── addition_integer.rs
```

When an operation has multiple implementation modules, create a directory for
the operation containing its own `mod.rs` and one file per implementation.
Decimal addition therefore uses:

```text
operations/
├── mod.rs
└── addition/
    ├── mod.rs
    ├── addition_decimal.rs
    ├── addition_decimal_double.rs
    ├── addition_decimal_float.rs
    └── addition_decimal_int.rs
```

Use `<operation>_<primary-type>.rs` for the primary-type implementation and
`<operation>_<primary-type>_<other-type>.rs` for mixed types. Do not create an
operation directory for a single implementation or leave several
implementations of the same operation loose in `operations/`.

Follow [the repository test layout](../tests.md) for all tests.

## Comparisons

Keep comparisons separate from arithmetic in the owning primitive or extension
`comparisons/` directory. Its `mod.rs` coordinates comparison modules and their
registration. Comparison modules define the compatibility relation between
operand types; they do not belong to the boolean primitive merely because their
result is a boolean.

Use the directory when a crate supports comparisons, even when it currently
has a single implementation module. Name implementation modules for the
relation they implement. Decimal comparisons therefore use:

```text
comparisons/
├── binary_float.rs
├── decimal.rs
├── decimal_double.rs
├── decimal_float.rs
├── decimal_integer.rs
└── mod.rs
```

Use `<primary-type>.rs` for the same-type relation and
`<primary-type>_<other-type>.rs` for a mixed-type relation. A private helper
module such as `binary_float.rs` may contain representation-level comparison
logic shared by multiple relations, but it must not register relations itself.

Register every compatible operand pair in its relation module. This keeps
cross-type rules, such as decimal and integer comparison, together with the
conversion logic they require. Extensions that compare qualified values must
likewise register their compatible subtype pairs and operand scales in their
comparison module; for example, linear measures register meter/centimeter and
other compatible-unit relations there.

Mixed comparisons must preserve the precision of both operand
representations. Do not lower a higher-precision operand to a less precise
representation merely to reuse that representation's native comparison.

Declare cross-type comparisons through the registry's name-based comparison
contract. A relation to a type supplied by another extension must activate
whether that type is registered before or after the extension declaring the
relation. Do not make comparison availability depend on extension registration
order or duplicate the relation in both participating extensions.
