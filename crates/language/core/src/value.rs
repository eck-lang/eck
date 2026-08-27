use std::{any::Any, sync::Arc};

use crate::{SubtypeId, TypeId, ValueType};

#[derive(Clone)]
pub struct Value {
    type_id: TypeId,
    subtype_id: Option<SubtypeId>,
    inner: Arc<dyn Any + Send + Sync>,
}

impl Value {
    pub fn new<T: Any + Send + Sync>(type_id: TypeId, value: T) -> Self {
        Self {
            type_id,
            subtype_id: None,
            inner: Arc::new(value),
        }
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn subtype_id(&self) -> Option<SubtypeId> {
        self.subtype_id
    }

    pub fn value_type(&self) -> ValueType {
        ValueType {
            base: self.type_id,
            subtype: self.subtype_id,
        }
    }

    pub fn with_subtype(mut self, subtype_id: Option<SubtypeId>) -> Self {
        self.subtype_id = subtype_id;
        self
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }
}
