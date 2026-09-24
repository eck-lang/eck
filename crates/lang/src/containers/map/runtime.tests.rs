use crate::{compile, execute, parse};

/// Verifies map literals, reads, writes, and missing values execute together.
#[test]
fn executes_map_storage_operations() {
    let registry = crate::default_registry().unwrap();
    let source = r#"
        let values = {10: "integer", "10": "string"}
        values[10] = "updated"
        print(values[10])
        print(values[11])
    "#;
    let parsed = parse(source).unwrap();
    let program = compile(&parsed, &registry).unwrap();

    execute(&program, &registry).unwrap();
}
