#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Remainder,
    Power,
}

/// An operator applied to one expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Negation,
}
