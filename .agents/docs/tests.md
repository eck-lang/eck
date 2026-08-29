# Test layout

## Unit tests

Keep unit tests next to the source module they verify. Every implementation
file has a sibling test file using the same stem and the `.tests.rs` suffix:

```text
src/foo.rs                    -> src/foo.tests.rs
src/registry/functions.rs     -> src/registry/functions.tests.rs
src/lib.rs                    -> src/lib.tests.rs
```

The implementation file declares its test module explicitly:

```rust
#[cfg(test)]
#[path = "foo.tests.rs"]
mod tests;
```

The test file imports the module under test with:

```rust
use super::*;
```

This keeps unit tests in the same crate, so they may verify private module
details when that is useful. A `foo.tests.rs` file should primarily test the
behaviour implemented by `foo.rs`.

Use `lib.tests.rs` for tests of the crate's public façade that necessarily
exercise several internal modules together. Do not use a generic `tests.rs`:
the filename must state which source file owns the tests.

## Test support

Shared test-support modules must be thoroughly commented. Document the purpose
and intended use of each helper, especially any builder, fixture, assertion,
or abstraction whose behaviour is not immediately apparent from its name and
signature. Explain relevant setup, assumptions, and constraints so test
authors can use the support code without having to reverse-engineer it.

## Integration tests

Place Rust integration tests under the crate-level `tests/` directory:

```text
crates/language/core/tests/registry.rs
```

Integration tests use the crate as an external consumer and therefore verify
only public APIs. Use them for cross-crate or end-to-end behaviour; keep
module-specific implementation tests in the matching `*.tests.rs` file.
