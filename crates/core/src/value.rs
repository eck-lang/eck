use std::{any::Any, sync::Arc};

use crate::{SubtypeId, TypeId, ValueType};

/// Contiguous runtime storage for an ECK array.
///
/// Each element remains a complete [`Value`], which preserves its concrete
/// base representation and optional subtype in unconstrained arrays.
///
/// The buffer grows and shrinks at both ends without moving the live elements.
/// `storage` is the whole allocation and the live elements occupy
/// `storage[start .. start + length]`, so the first logical element is a
/// contiguous slice starting at `start` and every consumer keeps reading one
/// ordinary pointer plus a length. Slots outside that window hold
/// [`Value::vacant_slot`] placeholders, which keeps every slot of the
/// allocation initialized and therefore safe to index and to drop.
///
/// Inserting at the front claims one unused slot before `start`, and removing
/// the first element advances `start`, instead of moving the remaining
/// elements. When the required end holds no unused slot the elements are moved
/// into a larger allocation, which keeps a sequence of insertions at either end
/// amortized `O(1)`, exactly like `push` on a plain vector.
///
/// The representation deliberately stays a bidirectionally growable contiguous
/// buffer rather than a segmented or circular deque: bulk numeric, slice, and
/// SIMD consumers need one undisjoint payload, so the live elements must never
/// be split across two memory regions.
pub struct ArrayValue {
    /// The whole allocation. Slots outside the live window are placeholders.
    storage: Vec<Value>,
    /// Index of the first live element inside `storage`.
    start: usize,
    /// Number of live elements.
    length: usize,
}

/// Smallest allocation a grown array receives.
///
/// The value keeps the first insertion into an empty array from allocating one
/// element at a time.
const MINIMUM_ARRAY_ALLOCATION: usize = 4;

impl ArrayValue {
    /// Creates an array from values already validated by the compiler.
    ///
    /// The elements become the whole allocation, so an array that is only
    /// appended to never reserves unused space in front of its first element.
    #[inline]
    pub fn new(elements: Vec<Value>) -> Self {
        let length = elements.len();
        Self {
            storage: elements,
            start: 0,
            length,
        }
    }

    /// Returns the number of live elements.
    #[inline]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Borrows every live element as one contiguous slice.
    #[inline]
    pub fn elements(&self) -> &[Value] {
        debug_assert!(self.start + self.length <= self.storage.len());
        &self.storage[self.start..self.start + self.length]
    }

    /// Mutably borrows every live element when the containing value is unique.
    #[inline]
    pub fn elements_mut(&mut self) -> &mut [Value] {
        debug_assert!(self.start + self.length <= self.storage.len());
        &mut self.storage[self.start..self.start + self.length]
    }

    /// Appends one value after the last element.
    ///
    /// The call claims the unused slot after the live window when one is
    /// available and otherwise grows the allocation, so its cost is amortized
    /// `O(1)`.
    #[inline]
    pub fn push(&mut self, value: Value) {
        if self.start + self.length == self.storage.len() {
            self.grow_for_back();
        }
        self.storage[self.start + self.length] = value;
        self.length += 1;
    }

    /// Removes and returns the last element, or `None` when the array is empty.
    ///
    /// The removed value is moved out of storage, so the operation transfers
    /// exactly one owner and never duplicates or clones an element.
    #[inline]
    pub fn pop(&mut self) -> Option<Value> {
        if self.length == 0 {
            return None;
        }
        self.length -= 1;
        let value = std::mem::replace(
            &mut self.storage[self.start + self.length],
            Value::vacant_slot(),
        );
        self.recenter_when_empty();
        Some(value)
    }

    /// Inserts one value before the first element.
    ///
    /// The call claims the unused slot before the live window when one is
    /// available and otherwise grows and recenters the allocation, so its cost
    /// is amortized `O(1)`.
    #[inline]
    pub fn unshift(&mut self, value: Value) {
        if self.start == 0 {
            self.grow_for_front();
        }
        self.start -= 1;
        self.storage[self.start] = value;
        self.length += 1;
    }

    /// Removes and returns the first element, or `None` when the array is empty.
    ///
    /// The removed value is moved out of storage and the remaining elements are
    /// left where they are, so the operation transfers exactly one owner and
    /// never moves the rest of the array.
    #[inline]
    pub fn shift(&mut self) -> Option<Value> {
        if self.length == 0 {
            return None;
        }
        let value = std::mem::replace(&mut self.storage[self.start], Value::vacant_slot());
        self.start += 1;
        self.length -= 1;
        self.recenter_when_empty();
        Some(value)
    }

    /// Grows the allocation so one more element fits after the live window.
    ///
    /// The window keeps its position in proportion to the allocation, so an
    /// array that is only appended to starts its window at the beginning of the
    /// new allocation and never pays for front space it does not use, while an
    /// array that also inserts in front keeps the share of room it already had
    /// there. Doubling the allocation keeps appending amortized `O(1)` per
    /// element, exactly like a plain vector.
    ///
    /// This is the only operation that moves more than one element, and it runs
    /// only when the back of the window is already the end of the allocation.
    /// Reusing free space that exists only in front of the window would move the
    /// same elements without creating any new room, so the buffer does not carry
    /// that extra path.
    fn grow_for_back(&mut self) {
        let new_capacity = self.next_capacity(self.length + 1);
        let new_start = match self.storage.len() {
            0 => 0,
            current_capacity => self.start * new_capacity / current_capacity,
        };
        self.reallocate(new_capacity, new_start);
    }

    /// Grows and recenters the allocation so one more element fits before the live window.
    ///
    /// Recentering hands half of the extra slots to each end, so the new
    /// allocation leaves room for many further front insertions and keeps them
    /// amortized `O(1)` while still serving the back.
    fn grow_for_front(&mut self) {
        let new_capacity = self.next_capacity(self.length + 4);
        self.reallocate(new_capacity, (new_capacity - self.length) / 2);
    }

    /// Returns the allocation size that holds at least `required` live slots.
    ///
    /// Doubling keeps both ends amortized `O(1)`, and the minimum keeps an
    /// empty array from reallocating for its first few insertions.
    fn next_capacity(&self, required: usize) -> usize {
        (self.storage.len() * 2)
            .max(required)
            .max(MINIMUM_ARRAY_ALLOCATION)
    }

    /// Moves the live elements into a freshly allocated buffer.
    ///
    /// Every slot of the new allocation starts as a placeholder, so no part of
    /// the buffer is uninitialized, and the live elements are moved rather than
    /// cloned out of the old allocation before it is released. The old
    /// allocation therefore drops only placeholders, and every live element
    /// keeps exactly one owner.
    fn reallocate(&mut self, new_capacity: usize, new_start: usize) {
        let mut storage = vec![Value::vacant_slot(); new_capacity];
        let old_storage = std::mem::take(&mut self.storage);
        for (index, value) in old_storage.into_iter().enumerate() {
            if index >= self.start && index < self.start + self.length {
                storage[new_start + index - self.start] = value;
            }
        }
        self.storage = storage;
        self.start = new_start;
    }

    /// Restores a window with room at both ends once the last element is removed.
    ///
    /// An empty array has no live element to keep contiguous, so reclaiming
    /// both ends is free and keeps a drained array from reallocating when it is
    /// filled again. The allocation itself is retained deliberately, so
    /// repeated `push`/`pop` or `unshift`/`shift` pairs at one end never
    /// reallocate either.
    fn recenter_when_empty(&mut self) {
        if self.length == 0 {
            self.start = self.storage.len() / 2;
        }
    }
}

impl Clone for ArrayValue {
    /// Copies the live elements into a fresh, exactly sized allocation.
    ///
    /// Cloning is the copy-on-write step of array mutation, so it must not pay
    /// for the unused slots of the source allocation.
    fn clone(&self) -> Self {
        Self::new(self.elements().to_vec())
    }
}

/// Number of bytes reserved for one payload stored directly inside a [`Value`].
const INLINE_PAYLOAD_SIZE: usize = 16;

/// Base type recorded by the placeholder that fills an unused array slot.
///
/// No registry ever allocates registry id zero, so a placeholder can never be
/// mistaken for a value of a registered type.
const VACANT_TYPE_ID: TypeId = TypeId {
    registry_id: 0,
    index: u32::MAX,
};

/// Reports that no queried payload type is the placeholder's payload type.
///
/// Installing this check makes every downcast of a placeholder fail, so an
/// unused array slot cannot be read as a value of any type.
fn no_payload_type_matches(_: std::any::TypeId) -> bool {
    false
}

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

    /// Creates the placeholder that fills an unused array slot.
    ///
    /// The placeholder owns no payload and installs a type check that never
    /// matches, so it cannot be read as a value of any type. Array storage uses
    /// it to keep every allocated slot initialized without tracking separately
    /// which slots hold live elements.
    #[inline]
    fn vacant_slot() -> Self {
        Self {
            type_id: VACANT_TYPE_ID,
            subtype_id: None,
            payload: PayloadStorage::Inline {
                bytes: InlinePayload([0; 2]),
                payload_type_check: no_payload_type_matches,
            },
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
