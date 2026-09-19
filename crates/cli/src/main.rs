use std::{env, fs, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("eck: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: eck <file.eck>".to_string())?;
    if !path.ends_with(".eck") {
        return Err("input file must use the .eck extension".into());
    }

    let source = fs::read_to_string(&path).map_err(|e| format!("cannot read `{path}`: {e}"))?;

    let registry = eck_lang::default_registry().map_err(|error| error.to_string())?;

    let ast = eck_lang::parse(&source).map_err(|e| e.to_string())?;
    let program = eck_lang::compile(&ast, &registry).map_err(|e| e.to_string())?;
    eck_lang::execute(&program, &registry).map_err(|e| e.to_string())?;
    Ok(())
}
