//! The central semantic catalogue for an ECK language instance.
//!
//! `Registry` keeps all lookup indices required during compilation and runtime.
//! Name-based lookups use hash maps for expected constant-time resolution;
//! resolved operators and functions use dense vectors so their IDs are cheap to
//! dereference during execution.

use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    BinaryOperator, BinaryOperatorDescriptor, FunctionDescriptor, FunctionId, OperatorId, Scale,
    SubtypeBinaryRule, SubtypeDescriptor, SubtypeId, TypeDescriptor, TypeId,
};

mod functions;
mod operators;
mod subtypes;
mod types;

static NEXT_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

/// Stores the language capabilities installed by ECK extensions.
///
/// This is intentionally one façade: type, subtype, operator, and function
/// resolution cooperate to answer a single semantic question about an ECK
/// expression. Its internal indices remain separated by concern in child
/// modules.
pub struct Registry {
    /// Distinguishes IDs allocated by different registry instances.
    registry_id: u64,
    /// Allocates compact, stable IDs for registered base types.
    next_type_id: u32,
    /// Allocates compact, stable IDs for registered subtypes.
    next_subtype_id: u32,

    /// IDs issued to extensions but not yet registered as base types.
    allocated_type_ids: HashSet<TypeId>,
    /// IDs issued to extensions but not yet registered as subtypes.
    allocated_subtype_ids: HashSet<SubtypeId>,

    /// Resolves a declared type name to its compact ID.
    types_by_name: HashMap<&'static str, TypeId>,
    /// Stores type descriptors keyed by ID.
    types: HashMap<TypeId, TypeDescriptor>,

    /// Resolves a semantic subtype name to its compact ID.
    subtypes_by_name: HashMap<&'static str, SubtypeId>,
    /// Resolves every registered literal suffix to its subtype.
    subtypes_by_suffix: HashMap<&'static str, SubtypeId>,
    /// Stores subtype descriptors keyed by ID.
    subtypes: HashMap<SubtypeId, SubtypeDescriptor>,

    /// Resolves subtype-aware arithmetic rules by operator and operand subtype.
    subtype_operator_index:
        HashMap<(BinaryOperator, Option<SubtypeId>, Option<SubtypeId>), SubtypeBinaryRule>,
    /// Resolves direct conversions between registered subtypes.
    subtype_conversion_index: HashMap<(SubtypeId, SubtypeId), Scale>,

    /// Resolves a base-type operation to a dense operator ID.
    operator_index: HashMap<(BinaryOperator, TypeId, TypeId), OperatorId>,
    /// Stores executable operator descriptors in ID order.
    operators: Vec<BinaryOperatorDescriptor>,

    /// Resolves a function name to its candidate overload IDs.
    functions_by_name: HashMap<&'static str, Vec<FunctionId>>,
    /// Stores executable function descriptors in ID order.
    functions: Vec<FunctionDescriptor>,

    /// Selects the base type for uncontextualized integer literals.
    default_integer: Option<TypeId>,
    /// Selects the base type for uncontextualized fractional literals.
    default_fractional: Option<TypeId>,
    /// Selects the base type for uncontextualized string literals.
    default_string: Option<TypeId>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            registry_id: NEXT_REGISTRY_ID.fetch_add(1, Ordering::Relaxed),
            next_type_id: 0,
            next_subtype_id: 0,
            allocated_type_ids: HashSet::new(),
            allocated_subtype_ids: HashSet::new(),
            types_by_name: HashMap::new(),
            types: HashMap::new(),
            subtypes_by_name: HashMap::new(),
            subtypes_by_suffix: HashMap::new(),
            subtypes: HashMap::new(),
            subtype_operator_index: HashMap::new(),
            subtype_conversion_index: HashMap::new(),
            operator_index: HashMap::new(),
            operators: Vec::new(),
            functions_by_name: HashMap::new(),
            functions: Vec::new(),
            default_integer: None,
            default_fractional: None,
            default_string: None,
        }
    }
}

impl Registry {
    /// Creates an empty registry with no language capabilities installed.
    ///
    /// Extensions must register types, operations, and defaults before the
    /// compiler can resolve source expressions that depend on them.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates the next unique ID for a base type descriptor.
    ///
    /// Callers allocate an ID before constructing the matching
    /// [`TypeDescriptor`], then pass both to [`Registry::register_type`].
    pub fn allocate_type_id(&mut self) -> TypeId {
        let id = TypeId {
            registry_id: self.registry_id,
            index: self.next_type_id,
        };
        self.next_type_id += 1;
        self.allocated_type_ids.insert(id);
        id
    }

    /// Allocates the next unique ID for a subtype descriptor.
    ///
    /// Callers allocate an ID before constructing the matching
    /// [`SubtypeDescriptor`], then pass both to [`Registry::register_subtype`].
    pub fn allocate_subtype_id(&mut self) -> SubtypeId {
        let id = SubtypeId {
            registry_id: self.registry_id,
            index: self.next_subtype_id,
        };
        self.next_subtype_id += 1;
        self.allocated_subtype_ids.insert(id);
        id
    }
}
