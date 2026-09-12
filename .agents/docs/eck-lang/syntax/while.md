# `while` loop syntax

ECK supports condition-controlled `while` loops:

```eck
while (condition) {
    // Loop body.
}
```

- The condition must produce a `bool` value and the complete condition must
  always be wrapped in parentheses.
- Braces around the loop body are mandatory.
- The condition is evaluated before every iteration. A condition that is false
  initially produces no iterations.
- The body is a lexical child scope; bindings declared inside it do not escape
  the loop.
- `break` exits the loop, while `continue` immediately starts the next
  condition check.

Unparenthesized conditions are invalid syntax:

```eck
while condition {
    // Invalid.
}
```
