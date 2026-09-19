use std::collections::VecDeque;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::semantic::{Registry, Value};

use super::*;

/// Allocates a scalar type ID suitable for opaque storage fixtures.
fn value_type() -> crate::semantic::TypeId {
    Registry::new().allocate_type_id()
}

/// Creates an integer payload used by ordering and pointer tests.
fn value(number: i64) -> Value {
    Value::new(value_type(), number)
}

/// Reads an integer fixture from an opaque value.
fn integer(value: &Value) -> i64 {
    *value
        .downcast_ref::<i64>()
        .expect("the fixture stores an integer")
}

/// Verifies insertion and removal at both ends preserve deque order.
#[test]
fn end_operations_preserve_order() {
    let mut storage = ArrayStorage::new(Vec::new());
    storage.push(value(10));
    storage.push(value(20));
    storage.unshift(value(5));

    assert_eq!(
        storage.elements().iter().map(integer).collect::<Vec<_>>(),
        [5, 10, 20]
    );
    assert_eq!(integer(&storage.shift().expect("a first value exists")), 5);
    assert_eq!(integer(&storage.pop().expect("a last value exists")), 20);
    assert_eq!(
        storage.elements().iter().map(integer).collect::<Vec<_>>(),
        [10]
    );
}

/// Verifies removals from an empty storage never read an uninitialized slot.
#[test]
fn empty_removals_return_none() {
    let mut storage = ArrayStorage::new(Vec::new());

    assert!(storage.pop().is_none());
    assert!(storage.shift().is_none());
}

/// Verifies front insertion uses existing front capacity without moving live values.
#[test]
fn front_capacity_is_reused_before_reallocation() {
    let mut storage = ArrayStorage::new((0..8).map(value).collect());
    storage.unshift(value(-1));
    let front_capacity = storage.start;
    let allocation = storage.slots.as_ptr();
    let addresses: Vec<*const Value> = storage
        .elements()
        .iter()
        .map(|element| element as *const Value)
        .collect();

    for number in 0..front_capacity {
        storage.unshift(value(-2 - number as i64));
    }

    assert_eq!(storage.slots.as_ptr(), allocation);
    for (index, address) in addresses.into_iter().enumerate() {
        assert_eq!(
            &storage.elements()[index + front_capacity] as *const Value,
            address
        );
    }
}

/// Verifies back insertion claims existing back capacity without moving values.
#[test]
fn back_capacity_is_reused_before_reallocation() {
    let mut storage = ArrayStorage::new(Vec::new());
    storage.unshift(value(0));
    let back_capacity = storage.slots.len() - storage.start - storage.length;
    let allocation = storage.slots.as_ptr();
    let address = &storage.elements()[0] as *const Value;

    for number in 1..=back_capacity {
        storage.push(value(number as i64));
    }

    assert_eq!(storage.slots.as_ptr(), allocation);
    assert_eq!(&storage.elements()[0] as *const Value, address);
}

/// Verifies repeated front growth remains geometric instead of reallocating per insertion.
#[test]
fn front_growth_is_geometric() {
    let mut storage = ArrayStorage::new(Vec::new());
    let mut allocations = 0;
    let mut previous = std::ptr::null();

    for number in 0..1000 {
        storage.unshift(value(number));
        if storage.slots.as_ptr() != previous {
            allocations += 1;
            previous = storage.slots.as_ptr();
        }
    }

    assert!(
        allocations <= 12,
        "front growth allocated {allocations} times"
    );
    assert_eq!(storage.length(), 1000);
}

/// Verifies draining recenters and reuses the retained allocation.
#[test]
fn draining_recenters_and_reuses_storage() {
    let mut storage = ArrayStorage::new((0..64).map(value).collect());
    let allocation = storage.slots.as_ptr();

    while storage.shift().is_some() {}

    assert_eq!(storage.length(), 0);
    assert_eq!(storage.slots.as_ptr(), allocation);
    assert!(storage.start > 0);
    storage.push(value(1));
    storage.unshift(value(0));
    assert_eq!(storage.slots.as_ptr(), allocation);
    assert_eq!(
        storage.elements().iter().map(integer).collect::<Vec<_>>(),
        [0, 1]
    );
}

/// A payload that records exactly how many times its owner is dropped.
struct DropCountingPayload {
    drops: Arc<AtomicUsize>,
    number: i64,
}

/// Creates a counted value whose payload also identifies its position.
fn counted_value(type_id: crate::semantic::TypeId, drops: &Arc<AtomicUsize>, number: i64) -> Value {
    Value::new(
        type_id,
        DropCountingPayload {
            drops: Arc::clone(drops),
            number,
        },
    )
}

/// Reads the position marker from a counted storage element.
fn counted_number(value: &Value) -> i64 {
    value
        .downcast_ref::<DropCountingPayload>()
        .expect("the fixture stores a counted payload")
        .number
}

/// Creates a full storage allocation containing counted sequential elements.
fn counted_storage(
    length: usize,
    type_id: crate::semantic::TypeId,
    drops: &Arc<AtomicUsize>,
) -> ArrayStorage {
    let mut elements = Vec::with_capacity(length);
    for number in 0..length {
        elements.push(counted_value(type_id, drops, number as i64));
    }
    ArrayStorage::new(elements)
}

impl Drop for DropCountingPayload {
    /// Increments the shared drop counter.
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Verifies moved and retained values are dropped exactly once.
#[test]
fn live_values_are_dropped_exactly_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let type_id = value_type();
    let mut storage = ArrayStorage::new(
        (0..8)
            .map(|_| {
                Value::new(
                    type_id,
                    DropCountingPayload {
                        drops: Arc::clone(&drops),
                        number: 0,
                    },
                )
            })
            .collect(),
    );

    drop(storage.shift().expect("a first value exists"));
    drop(storage.pop().expect("a last value exists"));
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    drop(storage);
    assert_eq!(drops.load(Ordering::SeqCst), 8);
}

/// Verifies moving live values during growth does not leak or double-drop them.
#[test]
fn reallocation_drops_each_live_value_exactly_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let type_id = value_type();
    let payload = || {
        Value::new(
            type_id,
            DropCountingPayload {
                drops: Arc::clone(&drops),
                number: 0,
            },
        )
    };
    let mut storage = ArrayStorage::new(vec![payload(), payload()]);
    storage.push(payload());
    storage.unshift(payload());

    drop(storage);
    assert_eq!(drops.load(Ordering::SeqCst), 4);
}

/// Verifies construction preserves the final allocation and cloning is compact.
#[test]
fn construction_and_clone_keep_the_expected_final_capacity() {
    let mut elements = Vec::with_capacity(32);
    elements.extend((0..8).map(value));
    let input_capacity = elements.capacity();
    let storage = ArrayStorage::new(elements);

    assert_eq!(storage.slots.len(), input_capacity);
    assert_eq!(
        storage.elements().iter().map(integer).collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );

    let clone = storage.clone();
    assert_eq!(clone.slots.len(), clone.length);
    assert_eq!(
        clone.elements().iter().map(integer).collect::<Vec<_>>(),
        (0..8).collect::<Vec<_>>()
    );
}

/// Verifies long shift/push cycles preserve order without growing the buffer.
#[test]
fn long_shift_push_cycles_recenter_without_growth_or_drop_loss() {
    const INITIAL_LENGTH: usize = 64;
    const CYCLES: usize = 4_096;

    let drops = Arc::new(AtomicUsize::new(0));
    let type_id = value_type();
    let mut storage = counted_storage(INITIAL_LENGTH, type_id, &drops);
    let capacity = storage.slots.len();
    let mut expected = VecDeque::from_iter(0..INITIAL_LENGTH as i64);

    for number in 0..CYCLES {
        let removed = storage.shift().expect("the cycle starts non-empty");
        assert_eq!(counted_number(&removed), expected.pop_front().unwrap());
        drop(removed);

        let inserted = INITIAL_LENGTH as i64 + number as i64;
        storage.push(counted_value(type_id, &drops, inserted));
        expected.push_back(inserted);

        assert_eq!(storage.slots.len(), capacity);
        assert_eq!(
            storage
                .elements()
                .iter()
                .map(counted_number)
                .collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );
    }

    assert_eq!(drops.load(Ordering::SeqCst), CYCLES);
    drop(storage);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        INITIAL_LENGTH + CYCLES,
        "every removed and retained payload must be dropped exactly once"
    );
}

/// Verifies long pop/unshift cycles preserve order without growing the buffer.
#[test]
fn long_pop_unshift_cycles_recenter_without_growth_or_drop_loss() {
    const INITIAL_LENGTH: usize = 64;
    const CYCLES: usize = 4_096;

    let drops = Arc::new(AtomicUsize::new(0));
    let type_id = value_type();
    let mut storage = counted_storage(INITIAL_LENGTH, type_id, &drops);
    let capacity = storage.slots.len();
    let mut expected = VecDeque::from_iter(0..INITIAL_LENGTH as i64);

    for number in 0..CYCLES {
        let removed = storage.pop().expect("the cycle starts non-empty");
        assert_eq!(counted_number(&removed), expected.pop_back().unwrap());
        drop(removed);

        let inserted = -(number as i64) - 1;
        storage.unshift(counted_value(type_id, &drops, inserted));
        expected.push_front(inserted);

        assert_eq!(storage.slots.len(), capacity);
        assert_eq!(
            storage
                .elements()
                .iter()
                .map(counted_number)
                .collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );
    }

    assert_eq!(drops.load(Ordering::SeqCst), CYCLES);
    drop(storage);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        INITIAL_LENGTH + CYCLES,
        "every removed and retained payload must be dropped exactly once"
    );
}

/// Verifies long mixed-end cycles reuse one bounded allocation and keep every value once.
#[test]
fn long_mixed_end_cycles_recenter_without_growth_or_drop_loss() {
    const INITIAL_LENGTH: usize = 64;
    const CYCLES: usize = 4_096;

    let drops = Arc::new(AtomicUsize::new(0));
    let type_id = value_type();
    let mut storage = counted_storage(INITIAL_LENGTH, type_id, &drops);
    let capacity = storage.slots.len();
    let mut expected = VecDeque::from_iter(0..INITIAL_LENGTH as i64);

    for cycle in 0..CYCLES {
        let removed = match cycle % 4 {
            0 | 2 => {
                let expected_number = expected.pop_front().unwrap();
                let removed = storage.shift().expect("the cycle starts non-empty");
                assert_eq!(counted_number(&removed), expected_number);
                removed
            }
            1 | 3 => {
                let expected_number = expected.pop_back().unwrap();
                let removed = storage.pop().expect("the cycle starts non-empty");
                assert_eq!(counted_number(&removed), expected_number);
                removed
            }
            _ => unreachable!("the remainder is always below four"),
        };
        drop(removed);

        let inserted = INITIAL_LENGTH as i64 + cycle as i64;
        match cycle % 4 {
            0 | 3 => {
                storage.push(counted_value(type_id, &drops, inserted));
                expected.push_back(inserted);
            }
            1 | 2 => {
                storage.unshift(counted_value(type_id, &drops, inserted));
                expected.push_front(inserted);
            }
            _ => unreachable!("the remainder is always below four"),
        }

        assert_eq!(storage.slots.len(), capacity);
        assert_eq!(storage.length(), expected.len());
        assert_eq!(
            storage
                .elements()
                .iter()
                .map(counted_number)
                .collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>()
        );
    }

    assert_eq!(drops.load(Ordering::SeqCst), CYCLES);
    drop(storage);
    assert_eq!(
        drops.load(Ordering::SeqCst),
        INITIAL_LENGTH + CYCLES,
        "every removed and retained payload must be dropped exactly once"
    );
}
