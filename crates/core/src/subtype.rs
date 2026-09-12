use crate::{ComparisonId, OperatorId, SubtypeId, TypeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueType {
    pub base: TypeId,
    pub subtype: Option<SubtypeId>,
}

/// Defines how qualified operands are scaled before a comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtypeComparisonRule {
    pub left_operand_scale: Scale,
    pub right_operand_scale: Scale,
}

impl SubtypeComparisonRule {
    pub const fn new() -> Self {
        Self {
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

impl Default for SubtypeComparisonRule {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedComparison {
    pub comparison: ComparisonId,
    pub output: ValueType,
    pub left_operand_scale: Scale,
    pub right_operand_scale: Scale,
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

/// Defines how a qualified right operand scales into a fraction of the left
/// operand for relative addition and subtraction.
///
/// A relative rule interprets `left_operand + right_operand` as
/// `left_operand + (left_operand * scaled_right_operand)` (and symmetrically
/// for subtraction), where `scaled_right_operand` is the right magnitude
/// divided by `right_operand_scale`. The result preserves the left operand
/// subtype: a plain left operand yields a plain result, while a qualified
/// left operand such as a length keeps its unit.
#[derive(Clone, Copy, Debug)]
pub struct SubtypeRelativeRule {
    pub right_operand_scale: Scale,
}

impl SubtypeRelativeRule {
    pub const fn new(right_operand_scale: Scale) -> Self {
        Self {
            right_operand_scale,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedBinaryOperator {
    pub operator: OperatorId,
    pub output: ValueType,
    pub left_operand_scale: Scale,
    pub right_operand_scale: Scale,
    /// Scales the right operand into a fraction of the left operand before
    /// combining them. `None` evaluates `scaled_left operator scaled_right`
    /// directly; `Some` first multiplies the left magnitude by the scaled
    /// right magnitude and then applies the operator to the left magnitude
    /// and that adjustment, so `100 - 50%` reads as `100 - (100 * 50 / 100)`.
    pub relative_adjustment: Option<Scale>,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedSubtypeConversion {
    pub output: ValueType,
    pub scale: Scale,
}
