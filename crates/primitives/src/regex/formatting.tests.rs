use language_core::{Extension, Registry};

use super::*;

/// Verifies that regex values format as their original literal.
#[test]
fn formats_regex_as_literal() {
    let mut registry = Registry::new();
    crate::RegexExtension.register(&mut registry).unwrap();

    let value = registry.parse_regex("/hello/g", None).unwrap();
    let formatted = format(&value).unwrap();
    assert_eq!(formatted, "/hello/g");
}
