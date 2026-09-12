# Conditional syntax

ECK supports `if`, one or more optional `else if` branches, and an optional
final `else` branch:

```eck
if (first_condition) {
    // Executed when the first condition is true.
} else if (second_condition) {
    // Executed when the second condition is true.
} else {
    // Executed when every condition is false.
}
```

- Every `if` and `else if` condition must produce a `bool` value.
- Always wrap each complete condition in parentheses, including a single
  literal, variable, or comparison.
- Every branch uses a brace-delimited block. `else if` and `else` are optional;
  `else` has no condition and must be the final branch.

Unparenthesized conditions are invalid syntax:

```eck
if condition {
    // Invalid.
}

if (first_condition) {
} else if second_condition {
    // Invalid.
}
```
