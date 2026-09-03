mod error;
mod lexer;
mod parser;

pub use error::ParseError;
pub use lexer::ECK_KEYWORDS;

use syntax::Program;

/// Lexes and parses one ECK source file into its syntax tree.
///
/// The function performs no semantic validation: type, operator, and name
/// resolution remain responsibilities of the compiler layer.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = lexer::lex(source)?;
    parser::Parser::new(tokens).parse_program()
}

#[cfg(test)]
#[path = "lib.tests.rs"]
mod tests;
