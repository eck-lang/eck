# `for` loop syntax

ECK currently supports ascending integer range loops:

```eck
for (item in start..end) {
    // Loop body.
}
```

- Parentheses around the complete loop clause and braces around the body are
  mandatory.
- `start` and `end` may be integer expressions and are evaluated once before
  iteration.
- The range includes `start` and excludes `end`. It advances by one; equal or
  reversed bounds produce no iterations.
- The loop variable exists only inside the loop and may optionally be written
  as `let item` in the loop clause.
- `break` exits the loop, while `continue` advances to the next value.
- Collection iteration and descending ranges are not currently supported.
