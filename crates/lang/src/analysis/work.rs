//! Deterministic relative work estimates, prepared once from resolved execution IR.

use crate::ir::{
    TypedBinaryExecutionPlan, TypedBlock, TypedComparisonPlan, TypedConversionPlan,
    TypedExpression, TypedExpressionKind, TypedProgram, TypedScalePlan, TypedStatement,
};
use crate::semantic::{BinaryOperator, ComparisonId, OperatorId, Registry, TypeId};

use super::ExecutionAnalysis;

/// Leaves the measured 590,000-unit array replay sequential and admits the
/// 753,000-unit arithmetic workload; see the automatic-parallelization benchmark.
pub const DEFAULT_PARALLELIZATION_THRESHOLD: u64 = 750_000;

/// Nonnegative relative work units, never a wall-clock duration.
///
/// Composition and trip-count multiplication saturate so even enormous regions
/// remain predictably above a finite threshold. Zero represents an empty block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkCost(u64);

impl WorkCost {
    /// Constructs an exact nonnegative relative-work summary, including cached calls.
    pub const fn from_units(units: u64) -> Self {
        Self(units)
    }

    /// Returns relative work units for configuration, diagnostics and comparisons.
    pub const fn units(self) -> u64 {
        self.0
    }

    /// Composes sequential work without wrapping at the representation limit.
    pub const fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Scales one logical execution by a trip count without wrapping.
    pub const fn saturating_mul(self, iterations: u64) -> Self {
        Self(self.0.saturating_mul(iterations))
    }
}

/// Central operation vocabulary; integer units preserve cheap, exact decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostClass {
    Trivial,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl CostClass {
    /// Maps stable relative classes to the V1 cost table.
    pub const fn work_cost(self) -> WorkCost {
        WorkCost::from_units(match self {
            Self::Trivial => 1,
            Self::Low => 5,
            Self::Medium => 10,
            Self::High => 30,
            Self::VeryHigh => 100,
        })
    }
}

/// Walks each region once and attaches iteration costs to the existing safety table.
pub(super) fn estimate(
    program: &TypedProgram,
    registry: &Registry,
    analysis: &mut ExecutionAnalysis,
) {
    WorkEstimator { registry, analysis }.statements(program.statements());
}

/// Owns compile-time traversal only; the executor consumes the stored scalar cost.
struct WorkEstimator<'analysis> {
    registry: &'analysis Registry,
    analysis: &'analysis mut ExecutionAnalysis,
}

impl WorkEstimator<'_> {
    /// Adds statements in source order without fixed lexical-block overhead.
    fn statements(&mut self, statements: &[TypedStatement]) -> WorkCost {
        statements
            .iter()
            .fold(WorkCost::default(), |cost, statement| {
                cost.saturating_add(self.statement(statement))
            })
    }

    /// Includes every nested lexical statement exactly once in its parent summary.
    fn block(&mut self, block: &TypedBlock) -> WorkCost {
        self.statements(&block.statements)
    }

    /// Estimates statements, selecting the heavier branch and scaling known ranges.
    fn statement(&mut self, statement: &TypedStatement) -> WorkCost {
        use CostClass::*;
        match statement {
            TypedStatement::VariableDeclaration { expression, .. }
            | TypedStatement::Assignment { expression, .. } => self
                .expression(expression)
                .saturating_add(Trivial.work_cost()),
            TypedStatement::IndexedAssignment {
                index, expression, ..
            } => self
                .expression(index)
                .saturating_add(self.expression(expression))
                .saturating_add(Medium.work_cost()),
            TypedStatement::MapIndexedAssignment {
                key, expression, ..
            } => self
                .expression(key)
                .saturating_add(self.expression(expression))
                .saturating_add(High.work_cost()),
            TypedStatement::Block(block) => self.block(block),
            TypedStatement::If {
                condition,
                body,
                else_body,
                ..
            } => {
                let condition = self.expression(condition);
                let then_cost = self.block(body);
                let else_cost = else_body
                    .as_ref()
                    .map_or(WorkCost::default(), |body| self.block(body));
                condition.saturating_add(then_cost.max(else_cost))
            }
            TypedStatement::For {
                start,
                end,
                body,
                span,
                ..
            } => {
                let bounds = self.expression(start).saturating_add(self.expression(end));
                let iteration = self.block(body).saturating_add(Low.work_cost());
                if let Some(plan) = self.analysis.loops.get_mut(&(span.start, span.end)) {
                    plan.cost_per_iteration = iteration;
                }
                let work = static_trip_count(start, end).map_or_else(
                    || iteration.saturating_add(VeryHigh.work_cost()),
                    |count| iteration.saturating_mul(count),
                );
                bounds.saturating_add(Low.work_cost()).saturating_add(work)
            }
            TypedStatement::While {
                condition, body, ..
            } => self
                .expression(condition)
                .saturating_add(self.block(body))
                .saturating_add(VeryHigh.work_cost()),
            TypedStatement::ForEach { source, body, .. } => self
                .expression(source)
                .saturating_add(self.block(body))
                .saturating_add(VeryHigh.work_cost()),
            TypedStatement::Expression(expression) => self.expression(expression),
            TypedStatement::Configuration { .. } => High.work_cost(),
            TypedStatement::Break { .. } | TypedStatement::Continue { .. } => Trivial.work_cost(),
        }
    }

    /// Estimates resolved child computations and the selected operation's own work.
    fn expression(&self, expression: &TypedExpression) -> WorkCost {
        use CostClass::*;
        use TypedExpressionKind::*;
        match &expression.kind {
            Literal(_) | Variable { .. } => Trivial.work_cost(),
            ArrayLiteral { elements } => {
                self.arguments(elements).saturating_add(Medium.work_cost())
            }
            MapLiteral { entries } => {
                entries.iter().fold(High.work_cost(), |cost, (key, value)| {
                    cost.saturating_add(self.expression(key))
                        .saturating_add(self.expression(value))
                })
            }
            ElementAccess { array, index, .. } => self
                .expression(array)
                .saturating_add(self.expression(index))
                .saturating_add(Medium.work_cost()),
            MapAccess { map, key, .. } => self
                .expression(map)
                .saturating_add(self.expression(key))
                .saturating_add(High.work_cost()),
            Binary {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
            } => self
                .expression(left_operand)
                .saturating_add(self.expression(right_operand))
                .saturating_add(self.binary_plan(resolution.operator, execution_plan)),
            DynamicBinary {
                dispatch,
                left_operand,
                right_operand,
                ..
            } => self
                .expression(left_operand)
                .saturating_add(self.expression(right_operand))
                .saturating_add(
                    dispatch
                        .plans
                        .iter()
                        .flatten()
                        .map(|plan| {
                            self.binary_plan(plan.resolution.operator, &plan.execution_plan)
                        })
                        .max()
                        .unwrap_or(High.work_cost()),
                ),
            Comparison {
                resolution,
                execution_plan,
                left_operand,
                right_operand,
            } => self
                .expression(left_operand)
                .saturating_add(self.expression(right_operand))
                .saturating_add(self.comparison_plan(resolution.comparison, execution_plan)),
            DynamicComparison {
                dispatch,
                left_operand,
                right_operand,
                ..
            } => self
                .expression(left_operand)
                .saturating_add(self.expression(right_operand))
                .saturating_add(
                    dispatch
                        .resolutions
                        .iter()
                        .flatten()
                        .map(|plan| self.comparison_plan(plan.resolution.comparison, plan))
                        .max()
                        .unwrap_or(High.work_cost()),
                ),
            DynamicNegation { dispatch, operand } => self.expression(operand).saturating_add(
                dispatch
                    .plans
                    .iter()
                    .flatten()
                    .map(|plan| {
                        self.binary_plan(
                            plan.binary.resolution.operator,
                            &plan.binary.execution_plan,
                        )
                    })
                    .max()
                    .unwrap_or(High.work_cost()),
            ),
            OpenBinary {
                left_operand,
                right_operand,
                ..
            }
            | OpenComparison {
                left_operand,
                right_operand,
                ..
            } => self
                .expression(left_operand)
                .saturating_add(self.expression(right_operand))
                .saturating_add(High.work_cost()),
            OpenNegation { operand, .. } => {
                self.expression(operand).saturating_add(High.work_cost())
            }
            NullCheck { operand, .. } | LogicalNot { operand } => {
                self.expression(operand).saturating_add(Low.work_cost())
            }
            Logical {
                left_operand,
                right_operand,
                ..
            } => self
                .expression(left_operand)
                .saturating_add(self.expression(right_operand))
                .saturating_add(Low.work_cost()),
            Convert {
                target_base,
                scale_plan,
                expression,
                ..
            } => self
                .expression(expression)
                .saturating_add(self.conversion(*target_base, scale_plan)),
            DynamicConvert {
                dispatch,
                expression,
            } => self.expression(expression).saturating_add(
                dispatch
                    .plans
                    .iter()
                    .flatten()
                    .map(|plan| self.conversion_plan(plan))
                    .max()
                    .unwrap_or(High.work_cost()),
            ),
            ElementStore { expression, .. } => self
                .expression(expression)
                .saturating_add(Medium.work_cost()),
            ArrayBoundary { expression, .. } => {
                self.expression(expression).saturating_add(High.work_cost())
            }
            ArrayMethod { arguments, .. } => {
                self.arguments(arguments).saturating_add(Medium.work_cost())
            }
            SourceAs { source, .. } => self.expression(source).saturating_add(Medium.work_cost()),
            RowField { row, .. } | DynamicRowField { row, .. } => {
                self.expression(row).saturating_add(Medium.work_cost())
            }
            Call {
                function,
                arguments,
                ..
            } => self
                .arguments(arguments)
                .saturating_add(self.function(*function)),
            Pipe {
                function,
                base,
                arguments,
            } => self
                .expression(base)
                .saturating_add(self.arguments(arguments))
                .saturating_add(self.function(*function)),
        }
    }

    /// Adds argument evaluation independently of callback parameter ordering.
    fn arguments(&self, arguments: &[TypedExpression]) -> WorkCost {
        arguments
            .iter()
            .fold(WorkCost::default(), |cost, argument| {
                cost.saturating_add(self.expression(argument))
            })
    }

    /// Reads one cached native summary without walking a function implementation.
    fn function(&self, function: crate::semantic::FunctionId) -> WorkCost {
        self.registry
            .function(function)
            .map_or(CostClass::High.work_cost(), |function| function.work_cost)
    }

    /// Distinguishes expensive representations using already resolved operand types.
    fn expensive_type(&self, type_id: TypeId) -> bool {
        self.registry
            .type_descriptor(type_id)
            .is_ok_and(|descriptor| matches!(descriptor.name, "decimal" | "bigint"))
    }

    /// Prices resolved arithmetic identities rather than the source spelling.
    fn operator(&self, operator: OperatorId) -> WorkCost {
        use CostClass::*;
        self.registry
            .operator(operator)
            .map_or(High.work_cost(), |descriptor| {
                if self.expensive_type(descriptor.left_operand_type)
                    || self.expensive_type(descriptor.right_operand_type)
                {
                    return if descriptor.operator == BinaryOperator::Power {
                        VeryHigh
                    } else {
                        High
                    }
                    .work_cost();
                }
                match descriptor.operator {
                    BinaryOperator::Addition | BinaryOperator::Subtraction => Low,
                    BinaryOperator::Multiplication => Medium,
                    BinaryOperator::Division | BinaryOperator::Remainder => High,
                    BinaryOperator::Power => VeryHigh,
                }
                .work_cost()
            })
    }

    /// Adds compiled operand scales and a possible relative adjustment to arithmetic.
    fn binary_plan(&self, operator: OperatorId, plan: &TypedBinaryExecutionPlan) -> WorkCost {
        self.operator(operator)
            .saturating_add(self.scale(&plan.left_operand_scale))
            .saturating_add(self.scale(&plan.right_operand_scale))
            .saturating_add(
                plan.relative_adjustment_operator
                    .map_or(WorkCost::default(), |operator| self.operator(operator)),
            )
    }

    /// Accounts for expensive comparison representations and compiled operand scales.
    fn comparison_plan(&self, comparison: ComparisonId, plan: &TypedComparisonPlan) -> WorkCost {
        let cost = self
            .registry
            .comparison(comparison)
            .map_or(CostClass::High, |descriptor| {
                if self.expensive_type(descriptor.left_operand_type)
                    || self.expensive_type(descriptor.right_operand_type)
                {
                    CostClass::High
                } else {
                    CostClass::Low
                }
            });
        cost.work_cost()
            .saturating_add(self.scale(&plan.left_operand_scale))
            .saturating_add(self.scale(&plan.right_operand_scale))
    }

    /// Counts only the scale steps actually present in the resolved conversion plan.
    fn scale(&self, plan: &TypedScalePlan) -> WorkCost {
        plan.numerator.iter().chain(plan.denominator.iter()).fold(
            WorkCost::default(),
            |cost, step| {
                cost.saturating_add(CostClass::Trivial.work_cost())
                    .saturating_add(self.operator(step.operator))
            },
        )
    }

    /// Prices an identity, simple cast or nontrivial resolved subtype conversion.
    fn conversion(&self, target_base: Option<TypeId>, scale: &TypedScalePlan) -> WorkCost {
        let class = if !scale.is_identity() {
            CostClass::High
        } else if target_base.is_some() {
            CostClass::Medium
        } else {
            CostClass::Trivial
        };
        class.work_cost().saturating_add(self.scale(scale))
    }

    /// Applies the same rule to each finite conversion candidate before taking its maximum.
    fn conversion_plan(&self, plan: &TypedConversionPlan) -> WorkCost {
        self.conversion(plan.target_base, &plan.scale_plan)
    }
}

/// Recognizes immediate signed range bounds without executing registry callbacks.
fn static_trip_count(start: &TypedExpression, end: &TypedExpression) -> Option<u64> {
    let start = literal_integer(start)?;
    let end = literal_integer(end)?;
    Some(u64::try_from(end.saturating_sub(start).max(0)).unwrap_or(u64::MAX))
}

/// Reads already parsed signed literal payloads; symbolic bounds remain unknown in V1.
fn literal_integer(expression: &TypedExpression) -> Option<i128> {
    let TypedExpressionKind::Literal(value) = &expression.kind else {
        return None;
    };
    value
        .downcast_ref::<i64>()
        .map(|value| i128::from(*value))
        .or_else(|| value.downcast_ref::<i128>().copied())
        .or_else(|| value.downcast_ref::<i32>().map(|value| i128::from(*value)))
        .or_else(|| value.downcast_ref::<i16>().map(|value| i128::from(*value)))
        .or_else(|| value.downcast_ref::<i8>().map(|value| i128::from(*value)))
}

#[cfg(test)]
#[path = "work.tests.rs"]
mod tests;
