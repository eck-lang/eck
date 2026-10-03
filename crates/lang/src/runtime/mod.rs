mod error;
mod runtime;

pub use error::RuntimeError;
pub use runtime::{ExecutionOptions, Executor, execute};
pub(crate) use runtime::{PendingArrayWrite, Runtime};
