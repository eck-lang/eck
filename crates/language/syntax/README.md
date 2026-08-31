# eck-syntax

`eck-syntax` defines the in-memory syntax representation of an ECK program.

It exposes an abstract syntax tree made up of:

- `Program`, a sequence of statements;
- `Statement`, including structural type declarations, frame declarations,
  relation definitions and bindings, scalar variables, standalone expressions,
  and `if` statements;
- `Block`, a brace-delimited statement sequence with its source span;
- `Expression`, including numbers, strings, variables, qualified field access,
  columnar frame literals, arithmetic, comparison and logical operations,
  conversions, and calls;
- `BinaryOperator` and `ComparisonOperator`, the available infix operators;
- `Span`, the start and end position of an element in the source text.

The crate contains syntax data structures only: it does not interpret or execute programs.
All supported source delimiters produce the same decoded string expression;
delimiter choice is a lexical concern and is not retained in the AST.
