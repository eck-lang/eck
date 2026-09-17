//! Unit tests for the opaque runtime value representation.

use std::collections::VecDeque;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::Registry;

/// A payload that is too large to live inside a value and records its drops.
///
/// The type is used to prove that a payload carrying a destructor keeps the
/// shared representation and is released exactly once across clones.
struct DropCountingPayload {
    drop_count: Arc<AtomicUsize>,
    contents: [u64; 4],
}

/// A payload that fills the inline buffer exactly without needing a destructor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SixteenBytePayload([u64; 2]);

impl Drop for DropCountingPayload {
    /// Records one release of this payload.
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
    }
}

/// Allocates a fresh registered base type from a throwaway registry.
fn base_type() -> TypeId {
    Registry::new().allocate_type_id()
}

/// Allocates a fresh registered subtype from a throwaway registry.
fn subtype() -> SubtypeId {
    Registry::new().allocate_subtype_id()
}

/// Reads one payload through a borrowed downcast.
fn integer_payload(value: &Value) -> i64 {
    *value
        .downcast_ref::<i64>()
        .expect("the value should hold an integer payload")
}

/// Proves that small numeric payloads round-trip without a shared allocation.
#[test]
fn small_payloads_round_trip_through_the_inline_buffer() {
    let type_id = base_type();
    let integer = Value::new(type_id, -7_i64);
    let float = Value::new(type_id, 1.5_f32);
    let double = Value::new(type_id, 2.5_f64);
    let flag = Value::new(type_id, true);
    let letter = Value::new(type_id, 'x');
    let wide = Value::new(type_id, SixteenBytePayload([7, 9]));

    assert_eq!(integer.downcast_ref::<i64>(), Some(&-7_i64));
    assert_eq!(float.downcast_ref::<f32>(), Some(&1.5_f32));
    assert_eq!(double.downcast_ref::<f64>(), Some(&2.5_f64));
    assert_eq!(flag.downcast_ref::<bool>(), Some(&true));
    assert_eq!(letter.downcast_ref::<char>(), Some(&'x'));
    assert_eq!(
        wide.downcast_ref::<SixteenBytePayload>(),
        Some(&SixteenBytePayload([7, 9]))
    );
}

/// Proves that equal-sized payload types are never interchangeable.
#[test]
fn downcast_rejects_another_type_of_the_same_size() {
    let type_id = base_type();
    let integer = Value::new(type_id, 7_i64);

    assert_eq!(integer.downcast_ref::<i64>(), Some(&7_i64));
    assert!(integer.downcast_ref::<u64>().is_none());
    assert!(integer.downcast_ref::<f64>().is_none());
    assert!(integer.downcast_ref::<[u8; 8]>().is_none());

    let mut mutable = integer.clone();
    assert!(mutable.downcast_mut::<u64>().is_none());
    assert!(mutable.downcast_mut::<f64>().is_none());
}

/// Proves that cloning an inline payload copies it instead of sharing it.
#[test]
fn cloned_inline_payloads_are_independent() {
    let type_id = base_type();
    let original = Value::new(type_id, 7_i64);
    let mut clone = original.clone();

    assert!(original.is_uniquely_owned());
    assert!(clone.is_uniquely_owned());

    *clone
        .downcast_mut::<i64>()
        .expect("the clone holds an integer payload") = 9;

    assert_eq!(integer_payload(&original), 7);
    assert_eq!(integer_payload(&clone), 9);
}

/// Proves that mutating an inline payload is visible through its own value.
#[test]
fn inline_payload_mutation_observes_the_new_value() {
    let type_id = base_type();
    let mut value = Value::new(type_id, 1_i64);

    *value
        .downcast_mut::<i64>()
        .expect("the value holds an integer payload") = 42;

    assert_eq!(integer_payload(&value), 42);
}

/// Proves that large payloads keep the shared representation.
#[test]
fn large_payloads_use_the_shared_representation() {
    let type_id = base_type();
    let payload = [1_u64, 2, 3, 4];
    let original = Value::new(type_id, payload);

    assert_eq!(original.downcast_ref::<[u64; 4]>(), Some(&payload));
    assert!(original.is_uniquely_owned());

    let clone = original.clone();
    assert!(!original.is_uniquely_owned());
    assert!(!clone.is_uniquely_owned());

    let mut shared = original.clone();
    assert!(shared.downcast_mut::<[u64; 4]>().is_none());
    drop(clone);
    drop(original);
    assert!(shared.downcast_mut::<[u64; 4]>().is_some());
}

/// Proves that a payload carrying a destructor is released exactly once.
#[test]
fn shared_payload_destructors_run_once() {
    let type_id = base_type();
    let drop_count = Arc::new(AtomicUsize::new(0));
    let value = Value::new(
        type_id,
        DropCountingPayload {
            drop_count: Arc::clone(&drop_count),
            contents: [0; 4],
        },
    );

    assert_eq!(drop_count.load(Ordering::SeqCst), 0);
    let clone = value.clone();
    assert_eq!(drop_count.load(Ordering::SeqCst), 0);
    assert_eq!(
        value
            .downcast_ref::<DropCountingPayload>()
            .expect("the value holds the counting payload")
            .contents,
        [0; 4]
    );

    drop(value);
    assert_eq!(drop_count.load(Ordering::SeqCst), 0);
    drop(clone);
    assert_eq!(drop_count.load(Ordering::SeqCst), 1);
}

/// Proves that subtype qualification keeps the payload and its base type.
#[test]
fn subtype_qualification_preserves_the_payload() {
    let type_id = base_type();
    let subtype_id = subtype();
    let value = Value::new(type_id, 3_i64).with_subtype(Some(subtype_id));

    assert_eq!(value.type_id(), type_id);
    assert_eq!(value.subtype_id(), Some(subtype_id));
    assert_eq!(
        value.value_type(),
        ValueType::qualified(type_id, subtype_id)
    );
    assert_eq!(integer_payload(&value), 3);
}

/// Proves that an unqualified value reports its plain value type.
#[test]
fn unqualified_value_reports_a_plain_value_type() {
    let type_id = base_type();
    let value = Value::new(type_id, 3_i64).with_subtype(None);

    assert_eq!(value.subtype_id(), None);
    assert_eq!(value.value_type(), ValueType::plain(type_id));
}

/// Proves that payloads without a destructor are stored inline.
#[test]
fn payloads_without_a_destructor_are_stored_inline() {
    assert!(can_be_stored_inline::<i64>());
    assert!(can_be_stored_inline::<f64>());
    assert!(can_be_stored_inline::<bool>());
    assert!(can_be_stored_inline::<SixteenBytePayload>());
    assert!(!can_be_stored_inline::<String>());
    assert!(!can_be_stored_inline::<[u64; 4]>());
    assert!(!can_be_stored_inline::<Arc<AtomicUsize>>());
}

/// Proves that over-aligned payloads keep working through the shared path.
#[test]
fn over_aligned_payloads_round_trip_through_the_shared_path() {
    let type_id = base_type();
    let value = Value::new(type_id, 123_456_789_012_345_678_901_u128);

    assert_eq!(
        value.downcast_ref::<u128>(),
        Some(&123_456_789_012_345_678_901_u128)
    );
}

/// Keeps the value representation small enough for the runtime's value stack.
///
/// The runtime stores values in contiguous stacks and copies them on every
/// push, pop, and binding access, so growing the representation is a hot-path
/// cost. A 64-byte element also keeps every stack slot cache-line sized;
/// shrinking the value below that measured slower on the numeric benchmarks
/// because consecutive elements then straddle cache lines. This guard reports
/// the regression instead of hiding it.
#[test]
fn value_representation_stays_within_one_cache_line() {
    assert!(
        std::mem::size_of::<Value>() <= 64,
        "Value grew to {} bytes, past the aligned size the runtime stack relies on",
        std::mem::size_of::<Value>()
    );
}

/// Builds one array element carrying an integer payload.
fn element(value: i64) -> Value {
    Value::new(base_type(), value)
}

/// Reads every live element of an array as its integer payload.
fn array_contents(array: &ArrayValue) -> Vec<i64> {
    array.elements().iter().map(integer_payload).collect()
}

/// A deterministic pseudo-random source for the mixed-operation model test.
struct Random(u64);

impl Random {
    /// Advances the generator and returns its next value.
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

/// Verifies both ends insert and remove elements in deque order.
#[test]
fn operations_at_both_ends_follow_deque_order() {
    let mut array = ArrayValue::new(Vec::new());

    array.push(element(10));
    array.push(element(20));
    array.unshift(element(5));
    assert_eq!(array_contents(&array), vec![5, 10, 20]);
    assert_eq!(array.length(), 3);

    let first = array.shift().expect("the array has a first element");
    let last = array.pop().expect("the array has a last element");
    assert_eq!(integer_payload(&first), 5);
    assert_eq!(integer_payload(&last), 20);
    assert_eq!(array_contents(&array), vec![10]);
}

/// Verifies removing from an empty array reports no element.
#[test]
fn removing_from_an_empty_array_reports_no_element() {
    let mut array = ArrayValue::new(Vec::new());

    assert!(array.pop().is_none());
    assert!(array.shift().is_none());

    array.push(element(1));
    assert!(array.pop().is_some());
    assert!(array.pop().is_none());
    assert!(array.shift().is_none());
}

/// Verifies a front insertion that has room claims it instead of moving elements.
///
/// The first insertion has to grow the allocation, because a literal array
/// starts exactly sized, and it recenters the window so free slots are left in
/// front of it. Every following insertion until those slots run out must claim
/// one of them, which neither reallocates nor moves an element.
#[test]
fn front_insertion_into_free_room_moves_no_element() {
    let mut array = ArrayValue::new((0..8).map(element).collect());
    array.unshift(element(-1));

    let free_front_slots = array.start;
    assert!(free_front_slots >= 1, "the growth must leave front room");
    let allocation = array.storage.as_ptr();
    let stored_addresses: Vec<*const Value> = {
        let elements = array.elements();
        (0..elements.len())
            .map(|index| &elements[index] as *const Value)
            .collect()
    };
    for step in 0..free_front_slots {
        array.unshift(element(-(step as i64) - 2));
    }

    assert_eq!(
        array.storage.as_ptr(),
        allocation,
        "free front room must be used before the allocation grows again"
    );
    for (index, address) in stored_addresses.iter().enumerate() {
        assert_eq!(
            &array.elements()[index + free_front_slots] as *const Value,
            *address,
            "the element that was at index {index} must not move"
        );
    }
    let expected: Vec<i64> = (0..free_front_slots)
        .rev()
        .map(|step| -(step as i64) - 2)
        .chain([-1])
        .chain(0..8)
        .collect();
    assert_eq!(array_contents(&array), expected);
}

/// Verifies repeated front insertions cost a logarithmic number of allocations.
///
/// A front insertion that moved every element, or that reallocated on every
/// call, would change the allocation once per insertion. Geometric growth with
/// recentering leaves front room for many insertions at once, so 1000
/// insertions must allocate a small constant number of times.
#[test]
fn repeated_front_insertions_grow_geometrically() {
    let mut array = ArrayValue::new(Vec::new());
    let mut allocations = 0;
    let mut previous = std::ptr::null();
    for value in 0..1000 {
        array.unshift(element(value));
        if array.storage.as_ptr() != previous {
            allocations += 1;
            previous = array.storage.as_ptr();
        }
    }

    assert_eq!(array.length(), 1000);
    assert!(
        allocations <= 12,
        "1000 front insertions allocated {allocations} times"
    );
    let expected: Vec<i64> = (0..1000).rev().collect();
    assert_eq!(array_contents(&array), expected);
}

/// Verifies repeated appends cost a logarithmic number of allocations.
#[test]
fn repeated_appends_grow_geometrically() {
    let mut array = ArrayValue::new(Vec::new());
    let mut allocations = 0;
    let mut previous = std::ptr::null();
    for value in 0..1000 {
        array.push(element(value));
        if array.storage.as_ptr() != previous {
            allocations += 1;
            previous = array.storage.as_ptr();
        }
    }

    assert!(
        allocations <= 12,
        "1000 appends allocated {allocations} times"
    );
    let expected: Vec<i64> = (0..1000).collect();
    assert_eq!(array_contents(&array), expected);
}

/// Verifies an array that is only appended to never reserves front space.
///
/// The growth policy redistributes the free slots in the proportion they already
/// hold, so an append-only array keeps its first element at the start of the
/// allocation instead of paying for front room it never uses.
#[test]
fn appending_only_never_reserves_front_space() {
    let mut array = ArrayValue::new(Vec::new());

    for value in 0..100 {
        array.push(element(value));
        assert_eq!(
            array.start, 0,
            "an append-only array must not reserve front space"
        );
    }
}

/// Verifies draining an array keeps its allocation and regains room at both ends.
#[test]
fn draining_keeps_the_allocation_and_recenters_the_window() {
    let mut array = ArrayValue::new((0..64).map(element).collect());
    let allocation = array.storage.as_ptr();

    while array.shift().is_some() {}

    assert_eq!(array.length(), 0);
    assert_eq!(
        array.storage.as_ptr(),
        allocation,
        "draining must not release the allocation"
    );
    assert!(
        array.start > 0,
        "an empty window is recentered so both ends have room"
    );
    array.push(element(1));
    array.unshift(element(0));
    assert_eq!(array.storage.as_ptr(), allocation);
    assert_eq!(array_contents(&array), vec![0, 1]);
}

/// Verifies the live window is one contiguous run of elements.
///
/// The removed push and shift operations are the only operations that change the
/// window, so probing a mixed sequence proves the payload a bulk consumer reads
/// stays contiguous from the first logical element to the last.
#[test]
fn mixed_end_operations_match_a_reference_model() {
    let mut array = ArrayValue::new(Vec::new());
    let mut reference: VecDeque<i64> = VecDeque::new();
    let mut random = Random(0x5eed);

    for step in 0..2000_i64 {
        match random.next() % 4 {
            0 => {
                array.push(element(step));
                reference.push_back(step);
            }
            1 => {
                array.unshift(element(-step));
                reference.push_front(-step);
            }
            2 => assert_eq!(
                array.pop().map(|value| integer_payload(&value)),
                reference.pop_back()
            ),
            _ => assert_eq!(
                array.shift().map(|value| integer_payload(&value)),
                reference.pop_front()
            ),
        }
        let expected: Vec<i64> = reference.iter().copied().collect();
        assert_eq!(array_contents(&array), expected, "after step {step}");
    }
}

/// Verifies the live elements of an array can be mutated in place.
#[test]
fn mutable_element_access_reaches_the_live_window() {
    let mut array = ArrayValue::new(vec![element(1), element(2)]);
    array.unshift(element(0));

    array.elements_mut()[1] = element(9);

    assert_eq!(array_contents(&array), vec![0, 9, 2]);
}

/// Verifies an unused slot can never be read as a value of any type.
#[test]
fn unused_slots_hold_a_placeholder_that_matches_no_type() {
    let mut array = ArrayValue::new((0..8).map(element).collect());
    array.unshift(element(-1));
    assert!(
        array.start >= 1,
        "the window must leave a front placeholder"
    );
    assert!(
        array.start + array.length < array.storage.len(),
        "the window must leave a back placeholder"
    );

    let front_placeholder = &array.storage[array.start - 1];
    let back_placeholder = &array.storage[array.start + array.length];
    for placeholder in [front_placeholder, back_placeholder] {
        assert!(placeholder.downcast_ref::<i64>().is_none());
        assert!(placeholder.downcast_ref::<Value>().is_none());
    }
}

/// Verifies a clone copies only the live elements into a compact allocation.
#[test]
fn cloning_copies_only_the_live_elements() {
    let mut array = ArrayValue::new(Vec::new());
    for value in 0..8 {
        array.push(element(value));
    }
    array.unshift(element(-1));

    let mut clone = array.clone();

    assert_eq!(clone.length(), 9);
    assert_eq!(clone.storage.len(), 9, "a clone must not copy unused slots");
    assert_eq!(clone.start, 0);
    assert_eq!(array_contents(&clone), array_contents(&array));

    clone.pop();
    let expected: Vec<i64> = std::iter::once(-1).chain(0..8).collect();
    assert_eq!(array_contents(&array), expected);
    assert_eq!(clone.length(), 8);
}

/// Verifies an end operation transfers exactly one owner of a removed element.
///
/// Each element here owns a drop-counting payload, so the test proves that a
/// removal moves the payload out of the array, that releasing it runs the
/// destructor once, and that the elements left in the array are released exactly
/// once when the array is dropped: no element is duplicated, dropped twice, or
/// leaked.
#[test]
fn removed_elements_are_released_exactly_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut array = ArrayValue::new(Vec::new());
    for _ in 0..8 {
        array.push(Value::new(
            base_type(),
            DropCountingPayload {
                drop_count: Arc::clone(&drops),
                contents: [0; 4],
            },
        ));
    }

    drop(array.shift().expect("the array has a first element"));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(array.pop().expect("the array has a last element"));
    assert_eq!(drops.load(Ordering::SeqCst), 2);

    drop(array);
    assert_eq!(drops.load(Ordering::SeqCst), 8);
}
