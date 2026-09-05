# Language use-case tests

Self-contained `.eckt` tests under `testing/use-cases/` verify built-in
behaviour through the complete CLI pipeline. They complement Rust unit tests:
a `*.tests.rs` file proves the operation implementation, while a `.eckt` file
proves the same behaviour survives parsing, registration, and printing.

The `.eckt` file format (title, description, `>>> source`, `<<< stdout`,
`<<< stderr`, `<<< exit`) is defined in `testing/README.md`. Follow it
exactly.

## Layout

The use-case directory hierarchy must mirror the implementation ownership
tree whenever possible. Its first component is the crate directory name
without the `eck-` package prefix, and its descendant directories reproduce
the source modules that own the behavior. Omit only the `src` directory and
Rust file names. Do not group tests by a separate test-only taxonomy.

Depart from the implementation hierarchy only when an explicit task
instruction requires a different layout.

For example, tests for
`crates/primitives/src/decimal/operations/division/` belong in
`testing/use-cases/primitives/decimal/operations/division/`. A case-specific
suffix may distinguish scenarios in the leaf filename, but the containing
directories must remain aligned with the implementation modules.

Mirror the crate operation layout from `crate-structure.md` so every test has
an obvious owner:

```text
testing/use-cases/primitives/<type>/operations/<operation>/<operation>_<type>_<case>.eckt
```

For example, decimal addition lives in:

```text
testing/use-cases/primitives/decimal/operations/addition/addition_decimal_scale.eckt
```

Integer subtypes keep their own level, mirroring
`crates/primitives/src/integer/<subtype>/`:

```text
testing/use-cases/primitives/integer/integer8/operations/addition/addition_integer8_boundary.eckt
```

Every same-type case in a file must use only the file's own type. Even when a
type supports mixed operations elsewhere, exponents and divisors in its
same-type cases must carry the same type annotation.

## Granularity

Keep one or two related cases per file. The filename, title, and description
must state precisely what is tested, for example
`addition_integer_zero_identity.eckt` for zero as the left and right identity.
Do not accumulate every combination of an operation in a single file.

## Titles and descriptions

Titles and descriptions must identify the language behavior protected by the
case. The description must explain the chosen inputs and the observable
semantic outcome they exercise, such as precision, scale preservation,
promotion, rounding, or truncation. Do not use generic statements about
running through the CLI pipeline, and do not merely repeat the title or
operation name.

## Ownership of mixed-type cases

A mixed-type combination belongs to the file of its result type, so no
combination is covered twice:

- `int + float` (result `float`) belongs to `addition_float`;
- `float + double` (result `double`) belongs to `addition_double`;
- `decimal + double` (result `decimal`) belongs to `addition_decimal`.
- `int8 + int32` (result `int32`) belongs to `addition_integer32`.
- `int8 + bigint` (result `bigint`) belongs to `addition_bigint`.

Same-type-only types such as unsigned integers own only their own
combinations. `bigint` instead promotes every narrower signed integer width,
extending the fixed-width promotion chain with `bigint` as the widest result.

## Workflow

1. Probe the behaviour with the CLI first (`cargo build -p eck-cli`, then
   `./target/debug/eck <file.eck>`) to fix the exact expected output.
2. Write the `.eckt` file with verified `<<< stdout` and `<<< stderr` sections.
3. Run the focused subset (`cargo test-all -- testing/use-cases/primitives/decimal`)
   and then the full suite (`cargo test-all`).
