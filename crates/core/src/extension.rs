use crate::{CoreError, Registry};

pub trait Extension {
    fn name(&self) -> &'static str;
    fn register(&self, registry: &mut Registry) -> Result<(), CoreError>;
}
