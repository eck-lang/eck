use std::io::{self, Write};

use language_core::{CoreError, ExecutionContext, Value};

pub(crate) fn print(
    context: &ExecutionContext<'_>,
    arguments: &[Value],
) -> Result<Option<Value>, CoreError> {
    let value = arguments
        .first()
        .ok_or_else(|| CoreError::Runtime("print expects one argument".into()))?;
    let rendered = context.format_value(value)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{rendered}").map_err(|e| CoreError::Runtime(e.to_string()))?;
    Ok(None)
}
