# Crate and Primitive Module Structure

ECK uses a modular monocrate for its tightly coupled language implementation.
These domains are Rust modules under `crates/lang/src`, not separate Cargo
packages. The CLI and LSP remain independent product packages.

Built-in primitives share `eck-lang::primitives` and each live in a dedicated
`src/primitives/<primitive>/` module. Standalone extensions remain separate
only when they are genuinely independently consumed.

Arrays are built-in containers, not primitives and not standard-library values.
One module, `eck-lang::containers::array`, owns the whole feature and keeps
one concern per file rather than one concern per layer:

| File | Array responsibility |
| --- | --- |
| `containers/array/mod.rs` | The façade: the public array surface and the one registration point, `ArrayExtension` |
| `containers/array/value.rs`, `containers/array/storage.rs` | `ArrayValue` and the contiguous storage that moves its elements |
| `containers/array/contract.rs` | The element contract every value crosses into storage through |
| `containers/array/end_operations.rs` | Adding and removing one value at either end, and element access |
| `containers/array/formatting.rs` | Rendering an array and the identity each element carries |
| `containers/array/compiler/` | Array literals, the storage boundary, element access, end operations, and the element flow state |
| `containers/array/runtime.rs` | Executing the compiled array expressions and operations |

Two pieces stay outside the module, because a value needs them before any array
runs:

* the vocabulary (`ArrayType`, `ArrayElementContract`, `ArrayEndOperation`) is
  canonically owned by `eck-lang::containers::array::types`; `semantic`
  re-exports it as a compatibility façade because `SemanticType` and `Value`
  carry array identities;
* the array variants of `TypedExpressionKind` stay declared in `eck-lang::ir`,
  the contract between the compiler and the runtime, while their payloads and
  every handler live in the array module.

`eck-lang::semantic::registry::bootstrap` registers `ArrayExtension` together
with every other built-in, so the array module plugs into the registry the way a
primitive does without introducing a scalar type.

Do not move array storage into `primitives` or `std`, and do not describe the
current end operations as registry-native functions: their current syntax is
implemented as compiler/runtime intrinsics.

Array code has one owner per concern, and a new array feature belongs to the
owner of its concern:

* the array vocabulary stays in `containers::array`, with one canonical
  definition and a semantic re-export;
* the payload, its storage, the element contract, the end operations, element
  access, and the formatting stay in `containers/array`;
* the source spellings and the static array semantics stay in
  `containers/array/compiler`, which resolves a destination before the runtime
  runs;
* the array IR nodes stay declared in `ir`, while their payload types and every
  handler stay in `containers/array`;
* the compiler driver keeps one `ArrayElementFlow` value as its only array
  state, and every control-flow join goes through that type's methods;
* the runtime keeps expression evaluation, local-slot flow, and diagnostics that
  name a binding rather than a payload, and delegates every array step to
  `containers/array/runtime.rs`.

The element contract is one rule with two halves, and both live in the array
module: `containers/array/compiler/contract.rs` proves when a store already
carries the declared representation, and `containers/array/contract.rs` applies
the contract to the value the runtime holds. Change them together.

Generic scalar dispatch keeps its execution classes explicit:
`compiler/finite_dispatch.rs` builds dense plans for enumerated complete-type
domains, while open IR sites resolve from concrete runtime identities and cache
prepared Registry plans. Do not use “dynamic dispatch” to conflate those two
paths with a dynamic binding contract.

Use `crates/lang/src/primitives/decimal` as the reference for a primitive that
defines a value type and multiple operation implementations. Include only the
modules required by the primitive or extension's capabilities.


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

`lib.rs` is the crate façade. In `crates/lang/src/primitives`, it declares the
primitive modules and re-exports their extension types. Each primitive's `mod.rs`
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
