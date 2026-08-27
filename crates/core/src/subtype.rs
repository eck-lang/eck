use crate::{OperatorId, SubtypeId, TypeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueType {
    pub base: TypeId,
    pub subtype: Option<SubtypeId>,
}

impl ValueType {
    pub const fn plain(base: TypeId) -> Self {
        Self {
            base,
            subtype: None,
        }
    }

    pub const fn qualified(base: TypeId, subtype: SubtypeId) -> Self {
        Self {
            base,
            subtype: Some(subtype),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale {
    pub numerator: u64,
    pub denominator: u64,
}

impl Scale {
    pub const IDENTITY: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    pub const fn integer(factor: u64) -> Self {
        Self {
            numerator: factor,
            denominator: 1,
        }
    }

    pub const fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    pub const fn is_identity(self) -> bool {
        self.numerator == 1 && self.denominator == 1
    }
}

#[derive(Clone)]
pub struct SubtypeDescriptor {
    pub id: SubtypeId,
    pub name: &'static str,
    pub suffixes: &'static [&'static str],
}

impl SubtypeDescriptor {
    pub fn canonical_suffix(&self) -> &'static str {
        self.suffixes.first().copied().unwrap_or(self.name)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SubtypeBinaryRule {
    pub output: Option<SubtypeId>,
    pub left_operand_scale: Scale,
    pub right_operand_scale: Scale,
}

impl SubtypeBinaryRule {
    pub const fn new(output: Option<SubtypeId>) -> Self {
        Self {
            output,
            left_operand_scale: Scale::IDENTITY,
            right_operand_scale: Scale::IDENTITY,
        }
    }

    pub const fn with_operand_scales(
        mut self,
        left_operand_scale: Scale,
        right_operand_scale: Scale,
    ) -> Self {
        self.left_operand_scale = left_operand_scale;
        self.right_operand_scale = right_operand_scale;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedBinaryOperator {
    pub operator: OperatorId,
    pub output: ValueType,
    pub left_operand_scale: Scale,
    pub right_operand_scale: Scale,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedSubtypeConversion {
    pub output: ValueType,
    pub scale: Scale,
}
