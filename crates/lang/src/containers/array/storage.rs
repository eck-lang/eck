use std::{
    mem::{ManuallyDrop, MaybeUninit},
    ptr, slice,
};

use crate::values::Value;

/// Minimum allocation used when an empty array first grows.
const MINIMUM_ARRAY_ALLOCATION: usize = 4;

/// Owns one contiguous, possibly recentered live window of array elements.
///
/// Only `slots[start .. start + length]` is initialized. The remaining slots
/// are deliberately uninitialized and are never read or dropped as `Value`s.
pub(super) struct ArrayStorage {
    /// Allocation containing the live window and unused front/back slots.
    slots: Vec<MaybeUninit<Value>>,
    /// Index of the first initialized element.
    start: usize,
    /// Number of initialized elements in the live window.
    length: usize,
}

impl ArrayStorage {
    /// Creates storage whose live window owns every value from `elements`.
    pub(super) fn new(elements: Vec<Value>) -> Self {
        let length = elements.len();
        let capacity = elements.capacity();
        let elements = ManuallyDrop::new(elements);
        // SAFETY: `Value` and `MaybeUninit<Value>` have the same size and
        // alignment. The `Vec<Value>` allocation is transferred without
        // changing its pointer, and the live prefix remains initialized while
        // the remaining capacity is represented as uninitialized slots.
        let slots = unsafe {
            Vec::from_raw_parts(
                elements.as_ptr().cast_mut().cast::<MaybeUninit<Value>>(),
                capacity,
                capacity,
            )
        };
        Self {
            slots,
            start: 0,
            length,
        }
    }

    /// Returns the number of initialized elements.
    #[inline]
    pub(super) fn length(&self) -> usize {
        self.length
    }

    /// Borrows the initialized window as one contiguous slice.
    #[inline]
    pub(super) fn elements(&self) -> &[Value] {
        debug_assert!(self.start + self.length <= self.slots.len());
        // SAFETY: the live window is initialized by `new`, `push`, and
        // `unshift`; removals reduce `length` before exposing the slice.
        unsafe { slice::from_raw_parts(self.slots.as_ptr().add(self.start).cast(), self.length) }
    }

    /// Mutably borrows the initialized window as one contiguous slice.
    #[inline]
    pub(super) fn elements_mut(&mut self) -> &mut [Value] {
        debug_assert!(self.start + self.length <= self.slots.len());
        // SAFETY: as in `elements`, and the exclusive borrow prevents any
        // other access while the returned slice is live.
        unsafe {
            slice::from_raw_parts_mut(self.slots.as_mut_ptr().add(self.start).cast(), self.length)
        }
    }

    /// Appends one value after the live window.
    #[inline]
    pub(super) fn push(&mut self, value: Value) {
        if self.start + self.length == self.slots.len() {
            self.grow_for_back();
        }
        self.slots[self.start + self.length].write(value);
        self.length += 1;
    }

    /// Removes and returns the last live value, if one exists.
    #[inline]
    pub(super) fn pop(&mut self) -> Option<Value> {
        if self.length == 0 {
            return None;
        }
        self.length -= 1;
        let index = self.start + self.length;
        // SAFETY: this index was inside the initialized window before length
        // was reduced, so reading it moves exactly one initialized `Value`.
        let value = unsafe { self.slots[index].assume_init_read() };
        self.recenter_when_empty();
        Some(value)
    }

    /// Inserts one value before the live window.
    #[inline]
    pub(super) fn unshift(&mut self, value: Value) {
        if self.start == 0 {
            self.grow_for_front();
        }
        self.start -= 1;
        self.slots[self.start].write(value);
        self.length += 1;
    }

    /// Removes and returns the first live value, if one exists.
    #[inline]
    pub(super) fn shift(&mut self) -> Option<Value> {
        if self.length == 0 {
            return None;
        }
        let index = self.start;
        // SAFETY: `index` is the first initialized slot of a non-empty window.
        let value = unsafe { self.slots[index].assume_init_read() };
        self.start += 1;
        self.length -= 1;
        self.recenter_when_empty();
        Some(value)
    }

    /// Grows and recenters the window so one value fits at the back.
    fn grow_for_back(&mut self) {
        if self.length < self.slots.len() {
            self.recenter_for_back();
            return;
        }
        let new_capacity = self.next_capacity(self.length + 1);
        let new_start = start_for_back(new_capacity, self.length);
        self.reallocate(new_capacity, new_start);
    }

    /// Grows and recenters the window so one value fits at the front.
    fn grow_for_front(&mut self) {
        if self.length < self.slots.len() {
            self.recenter_for_front();
            return;
        }
        let new_capacity = self.next_capacity(self.length + 1);
        let new_start = start_for_front(new_capacity, self.length);
        self.reallocate(new_capacity, new_start);
    }

    /// Returns a geometrically grown allocation large enough for `required`.
    fn next_capacity(&self, required: usize) -> usize {
        (self.slots.len() * 2)
            .max(required)
            .max(MINIMUM_ARRAY_ALLOCATION)
    }

    /// Moves initialized values into a fresh allocation without cloning them.
    fn reallocate(&mut self, new_capacity: usize, new_start: usize) {
        let mut new_slots = uninitialized_slots(new_capacity);
        let old_slots = std::mem::take(&mut self.slots);
        for offset in 0..self.length {
            // SAFETY: every offset in the old live window is initialized, and
            // the new allocation has `new_start + offset` available.
            let value = unsafe { old_slots[self.start + offset].assume_init_read() };
            new_slots[new_start + offset].write(value);
        }
        self.slots = new_slots;
        self.start = new_start;
    }

    /// Recenters a non-full window while leaving one slot available at the back.
    fn recenter_for_back(&mut self) {
        debug_assert!(self.length < self.slots.len());
        self.recenter(start_for_back(self.slots.len(), self.length));
    }

    /// Recenters a non-full window while leaving one slot available at the front.
    fn recenter_for_front(&mut self) {
        debug_assert!(self.length < self.slots.len());
        self.recenter(start_for_front(self.slots.len(), self.length));
    }

    /// Moves the live window within the same allocation without cloning values.
    fn recenter(&mut self, new_start: usize) {
        debug_assert!(new_start + self.length <= self.slots.len());
        if self.start == new_start {
            return;
        }
        // SAFETY: both ranges lie within the allocation, the source is the
        // initialized live window, and `ptr::copy` handles overlap. The source
        // slots become logically uninitialized after the move; `MaybeUninit`
        // never drops their old bit patterns.
        unsafe {
            ptr::copy(
                self.slots.as_ptr().add(self.start),
                self.slots.as_mut_ptr().add(new_start),
                self.length,
            );
        }
        self.start = new_start;
    }

    /// Recenters an empty window so both ends can be reused.
    fn recenter_when_empty(&mut self) {
        if self.length == 0 {
            self.start = self.slots.len() / 2;
        }
    }
}

impl Clone for ArrayStorage {
    /// Clones the live window directly into one compact allocation.
    fn clone(&self) -> Self {
        let mut slots = uninitialized_slots(self.length);
        for (offset, slot) in slots.iter_mut().enumerate() {
            // SAFETY: every offset in the live window is initialized.
            let value = unsafe { (&*self.slots[self.start + offset].as_ptr()).clone() };
            slot.write(value);
        }
        Self {
            slots,
            start: 0,
            length: self.length,
        }
    }
}

impl Drop for ArrayStorage {
    /// Drops exactly the initialized values in the live window.
    fn drop(&mut self) {
        for index in self.start..self.start + self.length {
            // SAFETY: the range is precisely the initialized live window.
            unsafe { self.slots[index].assume_init_drop() };
        }
    }
}

/// Allocates `capacity` uninitialized slots for `Value` objects.
fn uninitialized_slots(capacity: usize) -> Vec<MaybeUninit<Value>> {
    let mut slots = Vec::with_capacity(capacity);
    // SAFETY: `MaybeUninit<Value>` may be uninitialized, and the vector length
    // records slots rather than initialized `Value`s. Only `write` and
    // `assume_init_read/drop` touch those slots as values.
    unsafe { slots.set_len(capacity) };
    slots
}

/// Chooses a live-window start that balances free slots after a back insertion.
fn start_for_back(capacity: usize, length: usize) -> usize {
    debug_assert!(length < capacity);
    (capacity - length - 1) / 2
}

/// Chooses a live-window start that balances free slots after a front insertion.
fn start_for_front(capacity: usize, length: usize) -> usize {
    debug_assert!(length < capacity);
    (capacity - length).div_ceil(2)
}

#[cfg(test)]
#[path = "storage.tests.rs"]
mod tests;
