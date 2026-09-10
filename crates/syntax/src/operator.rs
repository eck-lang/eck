#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Remainder,
    Power,
}

/// An operator that compares two expressions and yields a boolean value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

/// An operator that combines boolean expressions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalOperator {
    And,
    Or,
}

/// An operator applied to one expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Negation,
}
