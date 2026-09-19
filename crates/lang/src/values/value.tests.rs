//! Unit tests for the opaque runtime value representation.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::{ArrayElementMode, Registry};

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
    assert_eq!(std::mem::size_of::<Value>(), 64);
}

/// Verifies an array value retains its exact element contract without a scalar ID.
#[test]
fn array_identity_preserves_the_complete_semantic_type() {
    let array_type = ArrayType {
        element: ValueType::qualified(base_type(), subtype()),
        element_mode: ArrayElementMode::Exact,
    };
    let value = Value::new_array(array_type, SixteenBytePayload([3, 5]));

    assert_eq!(value.semantic_type(), SemanticType::Array(array_type));
    assert_eq!(value.array_type(), Some(array_type));
    assert_eq!(value.scalar_type(), None);
    assert!(value.is_uniquely_owned());
    assert_eq!(
        value.downcast_ref::<SixteenBytePayload>(),
        Some(&SixteenBytePayload([3, 5]))
    );
}

/// Verifies scalar-only accessors reject arrays instead of fabricating a type ID.
#[test]
fn scalar_accessors_reject_array_identities() {
    let array_type = ArrayType {
        element: ValueType::plain(base_type()),
        element_mode: ArrayElementMode::AdaptiveInt,
    };
    let value = Value::new_array(array_type, 7_u64);

    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| value.type_id())).is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| value.subtype_id())).is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| value.value_type())).is_err());
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| value
            .clone()
            .with_subtype(None)))
        .is_err()
    );
}

/// Verifies array identity ownership does not affect payload copy-on-write state.
#[test]
fn array_identity_is_not_counted_as_payload_ownership() {
    let array_type = ArrayType {
        element: ValueType::plain(base_type()),
        element_mode: ArrayElementMode::Exact,
    };
    let value = Value::new_array(array_type, SixteenBytePayload([11, 13]));
    let clone = value.clone();

    assert!(value.is_uniquely_owned());
    assert!(clone.is_uniquely_owned());
}
