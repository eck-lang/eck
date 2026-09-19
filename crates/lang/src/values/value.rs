use std::{any::Any, sync::Arc};

use crate::semantic::{ArrayType, SemanticType, SubtypeId, TypeId, ValueType};

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

/// Identifies whether a value is a scalar or an array without assigning arrays a
/// registry [`TypeId`].
#[derive(Clone)]
enum ValueIdentity {
    /// A scalar base type and optional subtype.
    Scalar(ValueType),
    /// A complete array element contract shared by value clones.
    Array(Arc<ArrayType>),
}

/// One opaque runtime value.
///
/// The identity and payload remain separate so the array element contract does
/// not affect payload copy-on-write ownership. The enum layout keeps this value
/// at the existing 64-byte size used by the runtime value stack.
#[derive(Clone)]
pub struct Value {
    identity: ValueIdentity,
    payload: PayloadStorage,
}

impl Value {
    /// Creates a value that owns `value` as its opaque payload.
    #[inline]
    pub fn new<T: Any + Send + Sync>(type_id: TypeId, value: T) -> Self {
        Self {
            identity: ValueIdentity::Scalar(ValueType::plain(type_id)),
            payload: PayloadStorage::from_owned(value),
        }
    }

    /// Creates an array value with its complete element contract.
    #[inline]
    pub fn new_array<T: Any + Send + Sync>(array_type: ArrayType, value: T) -> Self {
        Self {
            identity: ValueIdentity::Array(Arc::new(array_type)),
            payload: PayloadStorage::from_owned(value),
        }
    }

    /// Returns the complete semantic shape of this value.
    #[inline]
    pub fn semantic_type(&self) -> SemanticType {
        match &self.identity {
            ValueIdentity::Scalar(value_type) => SemanticType::Scalar(*value_type),
            ValueIdentity::Array(array_type) => SemanticType::Array(**array_type),
        }
    }

    /// Returns the scalar type, or `None` for an array value.
    #[inline]
    pub fn scalar_type(&self) -> Option<ValueType> {
        match self.identity {
            ValueIdentity::Scalar(value_type) => Some(value_type),
            ValueIdentity::Array(_) => None,
        }
    }

    /// Returns the complete array element contract, or `None` for a scalar.
    #[inline]
    pub fn array_type(&self) -> Option<ArrayType> {
        match &self.identity {
            ValueIdentity::Scalar(_) => None,
            ValueIdentity::Array(array_type) => Some(**array_type),
        }
    }

    /// Returns the registered base type of this scalar value.
    ///
    /// Array identities have no scalar base type and are rejected with a stable
    /// panic instead of being assigned a fabricated registry ID.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.scalar_identity().base
    }

    /// Returns the qualified subtype of this scalar value, when it has one.
    ///
    /// Array identities have no scalar subtype and are rejected with the same
    /// deterministic scalar-only contract as [`Self::type_id`].
    #[inline]
    pub fn subtype_id(&self) -> Option<SubtypeId> {
        self.scalar_identity().subtype
    }

    /// Returns the base type and optional subtype of this scalar value.
    ///
    /// Array identities are rejected because arrays do not have scalar
    /// [`ValueType`] values.
    #[inline]
    pub fn value_type(&self) -> ValueType {
        self.scalar_identity()
    }

    /// Returns this scalar value qualified with `subtype_id`.
    ///
    /// Array identities cannot be qualified and are rejected deterministically.
    #[inline]
    pub fn with_subtype(mut self, subtype_id: Option<SubtypeId>) -> Self {
        match &mut self.identity {
            ValueIdentity::Scalar(value_type) => value_type.subtype = subtype_id,
            ValueIdentity::Array(_) => panic!("cannot qualify an array value"),
        }
        self
    }

    /// Returns the scalar identity or rejects an array identity.
    #[inline]
    fn scalar_identity(&self) -> ValueType {
        match self.identity {
            ValueIdentity::Scalar(value_type) => value_type,
            ValueIdentity::Array(_) => panic!("array value has no scalar identity"),
        }
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
