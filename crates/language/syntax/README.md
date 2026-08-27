# eck-syntax

`eck-syntax` defines the in-memory syntax representation of an ECK program.

It exposes an abstract syntax tree made up of:

- `Program`, a sequence of statements;
- `Statement`, either a variable declaration or a standalone expression;
- `Expression`, including numbers, strings, variables, binary operations, conversions, and calls;
- `BinaryOperator`, the available arithmetic operators;
- `Span`, the start and end position of an element in the source text.

The crate contains syntax data structures only: it does not interpret or execute programs.
