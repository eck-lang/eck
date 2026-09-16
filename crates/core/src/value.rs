use std::{any::Any, sync::Arc};

use crate::{SubtypeId, TypeId, ValueType};

/// Contiguous runtime storage for an ECK array.
///
/// Each element remains a complete [`Value`], which preserves its concrete
/// base representation and optional subtype in unconstrained arrays.
#[derive(Clone)]
pub struct ArrayValue {
    elements: Vec<Value>,
}

impl ArrayValue {
    /// Creates an array from values already validated by the compiler.
    #[inline]
    pub fn new(elements: Vec<Value>) -> Self {
        Self { elements }
    }

    /// Borrows every stored element as one contiguous slice.
    #[inline]
    pub fn elements(&self) -> &[Value] {
        &self.elements
    }

    /// Mutably borrows every stored element when the containing value is unique.
    #[inline]
    pub fn elements_mut(&mut self) -> &mut [Value] {
        &mut self.elements
    }
}

/// Number of bytes reserved for one payload stored directly inside a [`Value`].
const INLINE_PAYLOAD_SIZE: usize = 16;

/// Raw storage for one payload that needs no destructor.
///
/// The buffer is sized and aligned for the ordinary numeric payloads, which
/// keeps their values out of the heap without enlarging the representation.
#[derive(Clone, Copy)]
struct InlinePayload([u64; 2]);

/// Compares one queried payload type against the type stored inline.
///
/// Each inline payload installs the monomorphized instance built for its own
/// type, so a match proves the buffer really holds the queried type. This
/// replaces a stored [`std::any::TypeId`], which is larger than the check it
/// would support.
type PayloadTypeCheck = fn(std::any::TypeId) -> bool;

/// Builds the inline type check belonging to `T`.
#[inline]
fn payload_type_check<T: Any>() -> PayloadTypeCheck {
    /// Reports whether the queried payload type is the checked type.
    #[inline]
    fn matches<T: Any>(queried: std::any::TypeId) -> bool {
        queried == std::any::TypeId::of::<T>()
    }
    matches::<T>
}

/// Storage for the single payload a [`Value`] carries.
///
/// Small payloads that need no destructor live directly inside the value, so
/// producing and copying ordinary numeric values never touches the heap.
/// Payloads that are larger, over-aligned, or drop-carrying keep the shared
/// allocation, which also preserves clone aliasing for those payloads.
#[derive(Clone)]
enum PayloadStorage {
    /// Stores one payload whose size and alignment fit [`InlinePayload`].
    ///
    /// `payload_type_check` proves the concrete Rust type written into the
    /// buffer so that a downcast for a different type of the same size is
    /// rejected instead of reinterpreting the stored bytes.
    Inline {
        bytes: InlinePayload,
        payload_type_check: PayloadTypeCheck,
    },
    /// Stores one payload behind a shared allocation.
    Shared(Arc<dyn Any + Send + Sync>),
}

impl PayloadStorage {
    /// Stores one owned payload in the cheapest representation that can hold it.
    #[inline]
    fn from_owned<T: Any + Send + Sync>(value: T) -> Self {
        if !can_be_stored_inline::<T>() {
            return Self::Shared(Arc::new(value));
        }
        let mut bytes = InlinePayload([0; 2]);
        // SAFETY: `T` needs no destructor, and its size and alignment fit the
        // inline buffer, so copying the payload bytes and then forgetting the
        // original value transfers exactly one owner into `bytes`.
        unsafe {
            std::ptr::copy_nonoverlapping(
                (&value as *const T).cast::<u8>(),
                bytes.0.as_mut_ptr().cast::<u8>(),
                std::mem::size_of::<T>(),
            );
        }
        std::mem::forget(value);
        Self::Inline {
            bytes,
            payload_type_check: payload_type_check::<T>(),
        }
    }

    /// Borrows the payload as `T` when that is the stored payload type.
    #[inline]
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        match self {
            Self::Inline {
                bytes,
                payload_type_check,
            } => {
                if !payload_type_check(std::any::TypeId::of::<T>()) {
                    return None;
                }
                // SAFETY: `payload_type_check` proves the buffer holds a value of
                // exactly `T`, and the buffer alignment covers every inline
                // payload.
                Some(unsafe { &*bytes.0.as_ptr().cast::<T>() })
            }
            Self::Shared(shared) => shared.downcast_ref::<T>(),
        }
    }

    /// Mutably borrows the payload as `T` when this storage owns it exclusively.
    #[inline]
    fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        match self {
            Self::Inline {
                bytes,
                payload_type_check,
            } => {
                if !payload_type_check(std::any::TypeId::of::<T>()) {
                    return None;
                }
                // SAFETY: as in `downcast_ref`. An inline payload is never
                // shared with another `Value`, because cloning copies the bytes
                // instead of sharing them.
                Some(unsafe { &mut *bytes.0.as_mut_ptr().cast::<T>() })
            }
            Self::Shared(shared) => Arc::get_mut(shared)?.downcast_mut::<T>(),
        }
    }

    /// Reports whether this storage is the only owner of its payload.
    #[inline]
    fn is_exclusively_owned(&self) -> bool {
        match self {
            Self::Inline { .. } => true,
            Self::Shared(shared) => Arc::strong_count(shared) == 1,
        }
    }
}

/// Reports whether `T` fits the inline payload buffer without a destructor.
#[inline]
fn can_be_stored_inline<T>() -> bool {
    !std::mem::needs_drop::<T>()
        && std::mem::size_of::<T>() <= INLINE_PAYLOAD_SIZE
        && std::mem::align_of::<T>() <= std::mem::align_of::<InlinePayload>()
}

/// One opaque runtime value.
///
/// A value records its registered base type and optional subtype, and owns a
/// single payload whose concrete Rust representation only the owning extension
/// knows. Payloads that need no destructor and fit the inline buffer are stored
/// inside the value, so ordinary numeric results do not allocate.
#[derive(Clone)]
pub struct Value {
    type_id: TypeId,
    subtype_id: Option<SubtypeId>,
    payload: PayloadStorage,
}

impl Value {
    /// Creates a value that owns `value` as its opaque payload.
    #[inline]
    pub fn new<T: Any + Send + Sync>(type_id: TypeId, value: T) -> Self {
        Self {
            type_id,
            subtype_id: None,
            payload: PayloadStorage::from_owned(value),
        }
    }

    /// Returns the registered base type of this value.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the qualified subtype of this value, when it has one.
    #[inline]
    pub fn subtype_id(&self) -> Option<SubtypeId> {
        self.subtype_id
    }

    /// Returns the base type and optional subtype as one value type.
    #[inline]
    pub fn value_type(&self) -> ValueType {
        ValueType {
            base: self.type_id,
            subtype: self.subtype_id,
        }
    }

    /// Returns this value qualified with `subtype_id`.
    #[inline]
    pub fn with_subtype(mut self, subtype_id: Option<SubtypeId>) -> Self {
        self.subtype_id = subtype_id;
        self
    }

    /// Borrows the payload as `T` when that is the stored payload type.
    #[inline]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.payload.downcast_ref::<T>()
    }

    /// Returns whether this value may mutate its opaque payload in place.
    ///
    /// Inline payloads are always safe to mutate, because cloning copies them
    /// and no other value can observe the change. Shared payloads report
    /// ownership only while a single [`Value`] holds the allocation, which
    /// preserves the immutable sharing semantics of cloned instances.
    #[inline]
    pub fn is_uniquely_owned(&self) -> bool {
        self.payload.is_exclusively_owned()
    }

    /// Returns a mutable payload reference when the payload can be mutated.
    ///
    /// Returns `None` for an incompatible payload type or a shared payload, so
    /// callers cannot mutate a value observed through another [`Value`] clone.
    /// An inline payload is always mutable because each value owns its own copy.
    #[inline]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.payload.downcast_mut::<T>()
    }
}

#[cfg(test)]
#[path = "value.tests.rs"]
mod tests;
