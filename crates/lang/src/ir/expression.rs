use std::sync::Arc;
use std::sync::Mutex;

use crate::semantic::{
    ArrayEndOperation, ArrayType, BinaryOperator, ComparisonOperator, FunctionId, IndexExtractor,
    OperatorId, Registry, ResolvedBinaryOperator, ResolvedComparison, ResolvedSubtypeConversion,
    SemanticType, TypeId, Value, ValueType,
};
use crate::syntax::{LogicalOperator, Span};

use crate::ir::binding::{BindingId, LocalVariableSlot};

/// One typed expression node with its inferred semantic output and source span.
///
/// `output` is `None` when the expression produces no value, such as a call to
/// a function that returns `void`.
#[derive(Clone)]
pub struct TypedExpression {
    pub kind: TypedExpressionKind,
    pub output: Option<SemanticType>,
    pub span: Span,
}

impl TypedExpression {
    /// Returns this expression's array contract when it produces an array.
    pub fn array_type(&self) -> Option<ArrayType> {
        match &self.output {
            Some(SemanticType::Array(array_type)) => Some((**array_type).clone()),
            _ => None,
        }
    }

    /// Returns the finite complete-type domain of a scalar expression when its
    /// identity is selected at runtime.
    ///
    /// A missing domain means that [`SemanticType::Scalar`] carries the exact
    /// identity. A present domain is immutable and shared by every nested plan
    /// that consumes the expression, so compile-time dispatch can enumerate
    /// complete `ValueType` candidates without falling back to subtype-only
    /// knowledge.
    pub fn complete_type_domain(&self) -> Option<Arc<CompleteTypeDomain>> {
        match &self.kind {
            TypedExpressionKind::Variable {
                complete_type_domain,
                ..
            } => complete_type_domain.clone(),
            TypedExpressionKind::ArrayMethod { result_domain, .. } => result_domain.clone(),
            TypedExpressionKind::ElementAccess { type_domain, .. } => type_domain.clone(),
            TypedExpressionKind::DynamicBinary { dispatch, .. } => dispatch.result_domain.clone(),
            TypedExpressionKind::DynamicNegation { dispatch, .. } => dispatch.result_domain.clone(),
            TypedExpressionKind::DynamicConvert { dispatch, .. } => dispatch.result_domain.clone(),
            TypedExpressionKind::Binary { execution_plan, .. } => {
                execution_plan.result_domain.clone()
            }
            TypedExpressionKind::ElementStore { expression, .. } => {
                expression.complete_type_domain()
            }
            TypedExpressionKind::ArrayBoundary { .. } => None,
            _ => None,
        }
    }

    /// Reports a produced value whose concrete semantic type is intentionally open.
    pub fn is_open_value(&self) -> bool {
        matches!(
            self.kind,
            TypedExpressionKind::OpenBinary { .. }
                | TypedExpressionKind::OpenNegation { .. }
                | TypedExpressionKind::ElementAccess { .. } if self.output.is_none()
        ) || matches!(
            self.kind,
            TypedExpressionKind::ArrayMethod { method, .. }
                if method.removes_element() && self.output.is_none()
        )
    }
}

/// Number of runtime identity pairs retained at one open dispatch site.
const OPEN_DISPATCH_CACHE_CAPACITY: usize = 4;

type RuntimeTypePair = (ValueType, ValueType);
type OpenBinaryEntries = Arc<Mutex<Vec<(RuntimeTypePair, TypedBinaryPlan)>>>;
type OpenComparisonEntries = Arc<Mutex<Vec<(RuntimeTypePair, TypedComparisonPlan)>>>;

/// One tiny per-site cache for genuinely open binary operations.
#[derive(Clone, Default)]
pub struct TypedOpenBinaryDispatch {
    entries: OpenBinaryEntries,
}

impl TypedOpenBinaryDispatch {
    /// Returns a cached plan for one runtime identity pair.
    pub(crate) fn get(&self, key: (ValueType, ValueType)) -> Option<TypedBinaryPlan> {
        self.entries
            .lock()
            .expect("open binary dispatch cache must not be poisoned")
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, plan)| plan.clone())
    }

    /// Adds one plan while keeping the cache allocation and lookup bounded.
    pub(crate) fn insert(&self, key: (ValueType, ValueType), plan: TypedBinaryPlan) {
        let mut entries = self
            .entries
            .lock()
            .expect("open binary dispatch cache must not be poisoned");
        if entries.len() == OPEN_DISPATCH_CACHE_CAPACITY {
            entries.remove(0);
        }
        entries.push((key, plan));
    }
}

/// One tiny per-site cache for genuinely open comparisons.
#[derive(Clone, Default)]
pub struct TypedOpenComparisonDispatch {
    entries: OpenComparisonEntries,
}

impl TypedOpenComparisonDispatch {
    /// Returns a cached comparison plan for one runtime identity pair.
    pub(crate) fn get(&self, key: (ValueType, ValueType)) -> Option<TypedComparisonPlan> {
        self.entries
            .lock()
            .expect("open comparison dispatch cache must not be poisoned")
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, plan)| plan.clone())
    }

    /// Adds one plan while keeping the cache allocation and lookup bounded.
    pub(crate) fn insert(&self, key: (ValueType, ValueType), plan: TypedComparisonPlan) {
        let mut entries = self
            .entries
            .lock()
            .expect("open comparison dispatch cache must not be poisoned");
        if entries.len() == OPEN_DISPATCH_CACHE_CAPACITY {
            entries.remove(0);
        }
        entries.push((key, plan));
    }
}

/// Describes the finite complete scalar identities a runtime value may carry.
///
/// `candidate_slots` maps the dense arithmetic key derived from a complete
/// [`ValueType`] to the compact candidate index used by an embedded plan. The
/// map is an immutable array rather than a hash table, so runtime dispatch
/// performs one key calculation and one bounds-checked slice access.
#[derive(Clone)]
pub struct CompleteTypeDomain {
    /// Candidate identities in the order used by compact dispatch plans.
    pub candidates: Arc<[ValueType]>,
    /// Dense-key to compact-candidate mapping for this registry shape.
    pub candidate_slots: Arc<[Option<usize>]>,
    /// Width of the optional subtype axis used to derive dense keys.
    pub dispatch_stride: usize,
}

impl CompleteTypeDomain {
    /// Builds an immutable complete-type domain from unique candidate identities.
    pub fn from_candidates(
        registry: &Registry,
        candidates: impl IntoIterator<Item = ValueType>,
    ) -> Arc<Self> {
        let dispatch_stride = registry.subtype_dispatch_width();
        let dispatch_width = registry.complete_type_dispatch_width(dispatch_stride);
        let mut complete_types = Vec::new();
        let mut candidate_slots = vec![None; dispatch_width];
        for candidate in candidates {
            let key = registry.complete_type_dispatch_key(candidate, dispatch_stride);
            let slot = candidate_slots
                .get_mut(key)
                .expect("a registered complete type must fit its dispatch domain");
            if slot.is_none() {
                *slot = Some(complete_types.len());
                complete_types.push(candidate);
            }
        }
        Arc::new(Self {
            candidates: complete_types.into_boxed_slice().into(),
            candidate_slots: candidate_slots.into_boxed_slice().into(),
            dispatch_stride,
        })
    }

    /// Returns the compact candidate index for an actual runtime identity.
    pub fn candidate_index(&self, registry: &Registry, value_type: ValueType) -> Option<usize> {
        let key = registry.complete_type_dispatch_key(value_type, self.dispatch_stride);
        self.candidate_slots.get(key).copied().flatten()
    }

    /// Returns the number of complete identities represented by this domain.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Reports whether this domain has no complete identities.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Builds the smallest immutable domain containing every input candidate.
    pub fn union(registry: &Registry, domains: &[Arc<Self>]) -> Arc<Self> {
        CompleteTypeDomain::from_candidates(
            registry,
            domains
                .iter()
                .flat_map(|domain| domain.candidates.iter().copied()),
        )
    }
}

/// One compile-time-resolved arithmetic step used to scale a magnitude.
///
/// `factor` comes from a resolved literal and `operator` selects the
/// multiplication or division that applies it.
#[derive(Clone)]
pub struct TypedScaleStep {
    pub operator: OperatorId,
    pub factor: Value,
}

/// Stores the multiplication and division needed for one subtype magnitude scale.
#[derive(Clone, Default)]
pub struct TypedScalePlan {
    pub numerator: Option<TypedScaleStep>,
    pub denominator: Option<TypedScaleStep>,
}

impl TypedScalePlan {
    /// Returns whether this plan leaves the source magnitude unchanged.
    pub fn is_identity(&self) -> bool {
        self.numerator.is_none() && self.denominator.is_none()
    }
}

/// One pre-resolved binary plan for a candidate pair of complete operand types.
#[derive(Clone)]
pub struct TypedBinaryPlan {
    pub resolution: ResolvedBinaryOperator,
    pub execution_plan: TypedBinaryExecutionPlan,
}

/// Pre-resolved binary plans for an operation whose operand complete types are
/// not fully known at compile time.
///
/// An unconstrained array element carries whatever subtype it was stored with,
/// so the compiler resolves the operation once per candidate subtype and stores
/// the plans here. Each operand contributes either one dispatch slot, when its
/// complete type is known, or [`Registry::subtype_dispatch_width`] slots, when
/// its subtype is selected by its runtime value. Plans are stored row-major and
/// a `None` entry marks a candidate pair the registry does not define, which the
/// runtime reports only if that pair is actually reached.
#[derive(Clone)]
pub struct TypedBinaryDispatch {
    pub left_domain: Arc<CompleteTypeDomain>,
    pub right_domain: Arc<CompleteTypeDomain>,
    pub plans: Vec<Option<TypedBinaryPlan>>,
    /// Complete result identities reachable from the valid candidate plans.
    pub result_domain: Option<Arc<CompleteTypeDomain>>,
}

/// Pre-resolved comparison relations for an operation whose operand complete
/// types are not fully known at compile time.
///
/// The layout mirrors [`TypedBinaryDispatch`]: each operand contributes one slot
/// when its complete type is known and the registry's subtype dispatch width
/// otherwise, and a `None` entry marks a candidate pair with no registered
/// relation.
#[derive(Clone)]
pub struct TypedComparisonDispatch {
    pub left_domain: Arc<CompleteTypeDomain>,
    pub right_domain: Arc<CompleteTypeDomain>,
    pub resolutions: Vec<Option<TypedComparisonPlan>>,
}

/// One pre-resolved comparison plan for a candidate pair of complete types.
#[derive(Clone)]
pub struct TypedComparisonPlan {
    pub resolution: ResolvedComparison,
    pub left_operand_scale: TypedScalePlan,
    pub right_operand_scale: TypedScalePlan,
}

/// One pre-resolved subtype conversion for a candidate source subtype.
#[derive(Clone)]
pub struct TypedConversionPlan {
    /// The conversion scale and the complete type it produces.
    pub conversion: ResolvedSubtypeConversion,
    /// An optional base type cast applied after scaling.
    pub target_base: Option<TypeId>,
    /// The compiler-resolved operators and factors that apply the scale.
    pub scale_plan: TypedScalePlan,
}

/// Pre-resolved conversions for a value whose source subtype varies at runtime.
///
/// The table is addressed by the source's subtype dispatch slot, so a dynamic
/// conversion selects its plan arithmetically instead of resolving a subtype
/// conversion during execution. A `None` entry marks a source subtype with no
/// registered conversion to the declared target.
#[derive(Clone)]
pub struct TypedConversionDispatch {
    /// Describes the declared target for diagnostics, such as `` `->to(mm)` ``.
    pub target_description: String,
    /// Complete source identities the conversion plans address.
    pub source_domain: Arc<CompleteTypeDomain>,
    pub plans: Vec<Option<TypedConversionPlan>>,
    /// Complete result identities reachable from the valid conversion plans.
    pub result_domain: Option<Arc<CompleteTypeDomain>>,
}

/// All pre-resolved dispatch needed to execute one typed binary expression.
///
/// It carries the operand scaling and optional relative adjustment so the
/// runtime never resolves a subtype conversion or operator during evaluation.
#[derive(Clone)]
pub struct TypedBinaryExecutionPlan {
    pub left_operand_scale: TypedScalePlan,
    pub right_operand_scale: TypedScalePlan,
    pub relative_adjustment_operator: Option<OperatorId>,
    /// Complete result identities that may be produced by context-aware steps.
    pub result_domain: Option<Arc<CompleteTypeDomain>>,
}

/// One pre-resolved unary negation plan for a complete operand identity.
#[derive(Clone)]
pub struct TypedUnaryNegationPlan {
    /// Zero with the same complete identity as the selected operand.
    pub zero: Value,
    /// The subtraction plan used to negate the operand.
    pub binary: TypedBinaryPlan,
}

/// Pre-resolved unary negation plans keyed by complete operand identity.
#[derive(Clone)]
pub struct TypedUnaryNegationDispatch {
    /// Complete operand identities the plans address.
    pub domain: Arc<CompleteTypeDomain>,
    pub plans: Vec<Option<TypedUnaryNegationPlan>>,
    /// Complete result identities reachable from the valid plans.
    pub result_domain: Option<Arc<CompleteTypeDomain>>,
}

/// Pre-resolved index extractors keyed by complete index identity.
#[derive(Clone)]
pub struct TypedIndexDispatch {
    /// Complete index identities the extractors address.
    pub domain: Arc<CompleteTypeDomain>,
    pub extractors: Vec<Option<IndexExtractor>>,
}

/// The resolved operation or value behind one [`TypedExpression`].
#[derive(Clone)]
pub enum TypedExpressionKind {
    /// A constant runtime value.
    Literal(Value),
    /// Reads a local binding from its statically allocated slot.
    Variable {
        name: String,
        binding: BindingId,
        slot: LocalVariableSlot,
        /// Complete scalar identities the binding may hold at runtime.
        complete_type_domain: Option<Arc<CompleteTypeDomain>>,
    },
    /// Builds one contiguous array payload from evaluated element expressions.
    ArrayLiteral { elements: Vec<TypedExpression> },
    /// Validates a dynamic array once and assigns the result a static contract.
    ArrayBoundary {
        array_type: ArrayType,
        expression: Box<TypedExpression>,
    },
    /// Applies one built-in end operation to the array stored in a local slot.
    ///
    /// The receiver is the array binding itself rather than a value, because the
    /// operation mutates that binding. An adding operation carries its single
    /// stored value in `arguments`; a removing operation has no argument. The
    /// stored value is prepared through the same element contract as any other
    /// value that enters the array, so insertion never has a coercion path of
    /// its own, and a removal produces the element together with whatever
    /// subtype it was stored with.
    ArrayMethod {
        method: ArrayEndOperation,
        binding: BindingId,
        slot: LocalVariableSlot,
        arguments: Vec<TypedExpression>,
        /// Complete scalar identities a removal may produce at runtime.
        result_domain: Option<Arc<CompleteTypeDomain>>,
        /// The value a removal produces when the array holds no element.
        ///
        /// The compiler resolves the language's null value once, so an empty
        /// removal never repeats a null-literal lookup or parse during
        /// execution. An adding operation produces no value at all and
        /// therefore carries `None`.
        empty_result: Option<Value>,
    },
    /// Applies an array element representation contract to one evaluated value.
    ///
    /// This node marks the exact point where a value crosses from the
    /// representation its expression produced into the representation the
    /// destination array declared. `element` is that destination contract: the
    /// complete element type every stored value must carry, whose base
    /// representation the runtime enforces and whose optional subtype the stored
    /// value keeps.
    ///
    /// The compiler inserts the node only when the crossing needs runtime work.
    /// An element it proved already carries the declared representation, and
    /// every adaptive `int` element, cross into storage without it. The node
    /// belongs to the stored expression, so a literal initializer and an indexed
    /// assignment reach their destination through the same prepared contract.
    ElementStore {
        element: ValueType,
        expression: Box<TypedExpression>,
    },
    /// Reads one zero-based element, using an immediate index when available.
    ElementAccess {
        array: Box<TypedExpression>,
        index: Box<TypedExpression>,
        constant_index: Option<usize>,
        index_extractor: IndexExtractor,
        /// Complete scalar identities a dynamic read may produce.
        type_domain: Option<Arc<CompleteTypeDomain>>,
        /// The complete index dispatch selected when the index identity varies.
        index_dispatch: Option<Box<TypedIndexDispatch>>,
    },
    /// Applies the plan selected by the operands' runtime complete types.
    ///
    /// The compiler pre-resolved every candidate subtype into `dispatch`, so the
    /// runtime selects a plan with one slot computation instead of resolving the
    /// subtype rules again.
    DynamicBinary {
        operator: BinaryOperator,
        dispatch: Box<TypedBinaryDispatch>,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Resolves and caches an operator from the operands' actual runtime identities.
    OpenBinary {
        operator: BinaryOperator,
        dispatch: TypedOpenBinaryDispatch,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Negates an operand through a complete-type dispatch table.
    DynamicNegation {
        dispatch: Box<TypedUnaryNegationDispatch>,
        operand: Box<TypedExpression>,
    },
    /// Resolves and caches negation when the operand identity is genuinely open.
    OpenNegation {
        dispatch: TypedOpenBinaryDispatch,
        operand: Box<TypedExpression>,
    },
    /// Applies the relation selected by the operands' runtime complete types.
    DynamicComparison {
        operator: ComparisonOperator,
        dispatch: Box<TypedComparisonDispatch>,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Resolves and caches a comparison from the operands' runtime identities.
    OpenComparison {
        operator: ComparisonOperator,
        dispatch: TypedOpenComparisonDispatch,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Applies a resolved binary operator, including operand scaling.
    Binary {
        resolution: ResolvedBinaryOperator,
        execution_plan: Box<TypedBinaryExecutionPlan>,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Applies a resolved comparison relation and yields a boolean.
    Comparison {
        resolution: ResolvedComparison,
        execution_plan: Box<TypedComparisonPlan>,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Tests an operand against null, either `==` or `!=`.
    NullCheck {
        operand: Box<TypedExpression>,
        equal: bool,
        boolean_type: TypeId,
    },
    /// Short-circuit `&&` or `||`.
    Logical {
        operator: LogicalOperator,
        left_operand: Box<TypedExpression>,
        right_operand: Box<TypedExpression>,
    },
    /// Boolean negation.
    LogicalNot { operand: Box<TypedExpression> },
    /// Applies a resolved subtype conversion and optional base-type change.
    Convert {
        conversion: ResolvedSubtypeConversion,
        target_base: Option<TypeId>,
        scale_plan: TypedScalePlan,
        expression: Box<TypedExpression>,
    },
    /// Applies the conversion the source's runtime subtype selects.
    DynamicConvert {
        dispatch: Box<TypedConversionDispatch>,
        expression: Box<TypedExpression>,
    },
    /// Calls a resolved function with positional arguments.
    Call {
        function: FunctionId,
        arguments: Vec<TypedExpression>,
    },
    /// Pipes a base value into a resolved function as its first argument.
    Pipe {
        function: FunctionId,
        base: Box<TypedExpression>,
        arguments: Vec<TypedExpression>,
    },
}

#[cfg(test)]
#[path = "expression.tests.rs"]
mod tests;
