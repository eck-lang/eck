# Code style

## Naming

Use complete, domain-specific English names for project-defined identifiers.
A reader should understand an identifier without having to expand an
abbreviation from its context.

- Spell out the words that describe an identifier's purpose. For example, use
  `equality` instead of `eq` and `partial_equality` instead of `partial_eq`.
- Do not introduce shortened words, initialisms, or local acronyms merely
  because their meaning can be inferred from nearby code.
- Choose a name that states the identifier's role and domain, not just its
  representation or position in an expression.

The only exception is an identifier imposed directly by the Rust core or a
third-party library. Keep that external identifier unchanged when the API
requires it, but do not carry its abbreviation into project-defined names.
