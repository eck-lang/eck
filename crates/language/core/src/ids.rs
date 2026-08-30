#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId {
    pub(crate) registry_id: u64,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubtypeId {
    pub(crate) registry_id: u64,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OperatorId {
    pub(crate) registry_id: u64,
    pub(crate) index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComparisonId {
    pub(crate) registry_id: u64,
    pub(crate) index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId {
    pub(crate) registry_id: u64,
    pub(crate) index: usize,
}
