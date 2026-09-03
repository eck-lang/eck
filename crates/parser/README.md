# eck-parser

`eck-parser` turns ECK source text into the syntax tree defined by
[`eck-syntax`](../language/syntax).

## Responsibilities

- Tokenize source text while preserving byte spans for diagnostics.
- Parse structural type and frame declarations, relation definitions and
  bindings, hand-written columnar frame literals, variable declarations,
  expressions, function calls, namespace-qualified calls, and `use` imports,
  brace-delimited `if` blocks, numeric suffixes, string literals, and postfix
  unit conversions.
- Apply ECK operator precedence and right-associative exponentiation.
- Return precise `ParseError` values for invalid lexical or syntactic input.

The crate does not resolve names, validate types, or execute programs. Those
responsibilities belong to the compiler and runtime layers.

Namespace imports accept `use Namespace`, namespace aliases, selective member
imports with independent aliases, wildcard member imports, and wildcard
namespace aliases. The parser preserves each imported identifier's span;
namespace existence and collisions are compiler concerns.

## Parsing flow

```text
source text
    -> lexer
    -> token stream
    -> parser
    -> eck-syntax::Program
```

Expression parsing uses Pratt binding powers: multiplication, division, and
remainder bind more tightly than addition and subtraction; `**` is
right-associative. A postfix conversion such as `distance->km` binds to the
expression immediately before it.

Relation predicates use qualified `role.field` expressions. Comparisons bind
more tightly than `&&`, which binds more tightly than `||`. Separate predicate
lines inside `on { ... }` remain distinct AST predicates and are implicitly
ANDed by relation semantics.

Frame literals use `frame { column: [values] }`. Every named list remains a
distinct column in the AST; the parser does not lower literals to row objects.

Statements inside an `if` block follow the same newline separation as the
top-level program. The final statement may end directly at `}`, which also
allows a compact single-statement form such as `if (ready) { print(ready) }`.

Single and double quotes delimit single-line strings. Backticks delimit strings
that may contain physical line breaks and preserve their indentation. The
lexer decodes delimiter escapes, common control escapes, and `\u{...}` Unicode
scalar escapes before producing the syntax-level string expression. Unknown or
malformed escapes are lexical errors.
