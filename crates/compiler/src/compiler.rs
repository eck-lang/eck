//! The source-to-IR driver.
//!
//! This module owns the compiler state and walks the source program, delegating
//! one concern per child module: imports, configuration, lexical scopes,
//! expressions, and the small conversion helpers they share.

use std::collections::HashMap;

use language_core::{
    BinaryOperator as CoreBinaryOperator, ComparisonOperator as CoreComparisonOperator, Registry,
    ValueType,
};
use syntax::{Block, Expression, Program, Statement};

use crate::CompileError;
use ir::{
    BindingId, BindingMetadata, LocalVariableSlot, TypedBlock, TypedExpression, TypedProgram,
    TypedRangePlan, TypedStatement,
};

mod configuration;
mod expressions;
mod helpers;
mod imports;
mod scopes;

use self::helpers::statement_span;

pub fn compile(program: &Program, registry: &Registry) -> Result<TypedProgram, CompileError> {
    Compiler {
        registry,
        variable_scopes: vec![HashMap::new()],
        next_local_slot: 0,
        bindings: Vec::new(),
        narrowed_bindings: HashMap::new(),
        loop_depth: 0,
        import_scopes: vec![ImportScope::default()],
    }
    .compile_program(program)
}

struct Compiler<'a> {
    registry: &'a Registry,
    variable_scopes: Vec<HashMap<String, LocalVariable>>,
    next_local_slot: usize,
    bindings: Vec<BindingMetadata>,
    narrowed_bindings: HashMap<BindingId, usize>,
    loop_depth: usize,
    import_scopes: Vec<ImportScope>,
}

/// Stores the semantic type and statically allocated storage of one local variable.
#[derive(Clone, Copy)]
struct LocalVariable {
    binding: BindingId,
    value_type: ValueType,
    slot: LocalVariableSlot,
    mutable: bool,
    nullable: bool,
}

/// Stores namespace and function imports for one lexical source scope.
#[derive(Default)]
struct ImportScope {
    namespaces: HashMap<String, NamespaceImport>,
    functions: HashMap<String, FunctionImport>,
}

/// Maps one local namespace name to its registered source namespace.
struct NamespaceImport {
    namespace: String,
}

/// Represents either one imported function or competing wildcard sources.
enum FunctionImport {
    Unique(FunctionImportSource),
    Ambiguous(Vec<FunctionImportSource>),
}

/// Identifies one namespace member introduced into lexical function scope.
#[derive(Clone)]
struct FunctionImportSource {
    namespace: String,
    member: String,
    wildcard: bool,
}

impl Compiler<'_> {
    /// Compiles every root statement into a typed program.
    ///
    /// Root-level `use` declarations apply to the root import scope and are
    /// consumed rather than lowered, so they never appear in the typed program.
    /// The result records the final local slot count and binding metadata.
    fn compile_program(&mut self, program: &Program) -> Result<TypedProgram, CompileError> {
        let mut statements = Vec::new();
        for statement in &program.statements {
            if let Statement::Use(declaration) = statement {
                self.compile_use_declaration(declaration)?;
                continue;
            }
            statements.push(self.compile_statement(statement)?);
        }
        Ok(TypedProgram {
            statements,
            local_slot_count: self.next_local_slot,
            bindings: std::mem::take(&mut self.bindings),
        })
    }

    /// Lowers one source statement into its typed counterpart.
    ///
    /// Root-level configuration is lowered here; `compile_block` rejects it
    /// when it appears inside a nested block.
    fn compile_statement(&mut self, statement: &Statement) -> Result<TypedStatement, CompileError> {
        match statement {
            Statement::Use(_) => unreachable!("use declarations are compiled out before IR"),
            Statement::Configuration { entries, span } => Ok(TypedStatement::Configuration {
                configuration_override: self.compile_configuration(entries)?,
                span: *span,
            }),
            Statement::VariableDeclaration {
                name,
                type_name,
                expression,
                span,
            } => self.compile_binding_declaration(
                name,
                Some(type_name),
                false,
                expression,
                false,
                *span,
            ),
            Statement::BindingDeclaration {
                kind,
                name,
                type_name,
                nullable,
                expression,
                span,
            } => self.compile_binding_declaration(
                name,
                type_name.as_deref(),
                *nullable,
                expression,
                matches!(kind, syntax::BindingKind::Let),
                *span,
            ),
            Statement::Assignment {
                name,
                expression,
                span,
            } => self.compile_assignment(name, expression, *span),
            Statement::Block(block) => Ok(TypedStatement::Block(self.compile_block(block)?)),
            Statement::If {
                condition,
                body,
                else_body,
                span,
            } => {
                let typed_condition = self.compile_boolean_condition(condition, "if")?;
                let narrowed_binding = self.non_null_narrowing_binding(condition);
                if let Some(binding) = narrowed_binding {
                    self.push_narrowing(binding);
                }
                let compiled_body = self.compile_block(body);
                if let Some(binding) = narrowed_binding {
                    self.pop_narrowing(binding);
                }
                let compiled_body = compiled_body?;
                Ok(TypedStatement::If {
                    condition: typed_condition,
                    body: compiled_body,
                    else_body: else_body
                        .as_ref()
                        .map(|body| self.compile_block(body))
                        .transpose()?,
                    span: *span,
                })
            }
            Statement::While {
                condition,
                body,
                span,
            } => {
                let typed_condition = self.compile_boolean_condition(condition, "while")?;
                let narrowed_binding = self.non_null_narrowing_binding(condition);
                if let Some(binding) = narrowed_binding {
                    self.push_narrowing(binding);
                }
                self.loop_depth += 1;
                let compiled_body = self.compile_block(body);
                self.loop_depth -= 1;
                if let Some(binding) = narrowed_binding {
                    self.pop_narrowing(binding);
                }
                Ok(TypedStatement::While {
                    condition: typed_condition,
                    body: compiled_body?,
                    span: *span,
                })
            }
            Statement::Expression(expression) => Ok(TypedStatement::Expression(
                self.compile_expression(expression, None)?,
            )),
            Statement::For {
                variable,
                start,
                end,
                body,
                span,
            } => self.compile_for_statement(variable, start, end, body, *span),
            Statement::Break { span } => {
                if self.loop_depth == 0 {
                    return Err(CompileError::new(
                        *span,
                        "`break` is only allowed inside a loop",
                    ));
                }
                Ok(TypedStatement::Break { span: *span })
            }
            Statement::Continue { span } => {
                if self.loop_depth == 0 {
                    return Err(CompileError::new(
                        *span,
                        "`continue` is only allowed inside a loop",
                    ));
                }
                Ok(TypedStatement::Continue { span: *span })
            }
            _ => Err(CompileError::new(
                statement_span(statement),
                "statement requires an optional language patch",
            )),
        }
    }

    /// Compiles a control-flow condition and enforces the configured boolean type.
    fn compile_boolean_condition(
        &mut self,
        condition: &Expression,
        construct_name: &str,
    ) -> Result<TypedExpression, CompileError> {
        let typed_condition = self.compile_expression(condition, None)?;
        self.require_non_nullable_expression(&typed_condition)?;
        let expected = ValueType::plain(
            self.registry
                .default_boolean()
                .map_err(|error| CompileError::core(condition.span(), error))?,
        );
        let actual = typed_condition.output.ok_or_else(|| {
            CompileError::new(
                condition.span(),
                format!(
                    "{construct_name} condition must produce `{}`, found `void`",
                    self.registry.value_type_name(expected)
                ),
            )
        })?;
        if actual != expected {
            return Err(CompileError::new(
                condition.span(),
                format!(
                    "{construct_name} condition must produce `{}`, found `{}`",
                    self.registry.value_type_name(expected),
                    self.registry.value_type_name(actual)
                ),
            ));
        }
        Ok(typed_condition)
    }
    /// Compiles one source binding after enforcing current-scope declaration rules.
    fn compile_binding_declaration(
        &mut self,
        name: &str,
        type_name: Option<&str>,
        nullable: bool,
        expression: &Expression,
        mutable: bool,
        span: syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        if self.variable_declared_in_current_scope(name) {
            return Err(CompileError::new(
                span,
                format!("binding `{name}` is already declared in this scope"),
            ));
        }
        let expected = type_name
            .map(|type_name| {
                self.registry
                    .type_by_name(type_name)
                    .ok_or_else(|| CompileError::new(span, format!("unknown type `{type_name}`")))
            })
            .transpose()?;
        if nullable
            && expected
                == Some(
                    self.registry
                        .default_null()
                        .map_err(|error| CompileError::core(span, error))?,
                )
        {
            return Err(CompileError::new(
                span,
                "`null?` is not a valid nullable type",
            ));
        }
        if nullable
            && expected.is_some_and(|expected| {
                ![
                    self.registry.default_integer().ok(),
                    self.registry.default_fractional().ok(),
                    self.registry.default_string().ok(),
                    self.registry.default_boolean().ok(),
                ]
                .into_iter()
                .flatten()
                .any(|supported| supported == expected)
            })
        {
            return Err(CompileError::new(
                span,
                "nullable types are currently limited to `int?`, `decimal?`, `string?`, and `bool?`",
            ));
        }
        let typed_expression = self.compile_expression(expression, expected)?;
        let actual = typed_expression.output.ok_or_else(|| {
            CompileError::new(
                expression.span(),
                "a void expression cannot initialize a binding",
            )
        })?;
        let initializer_is_null = matches!(expression, Expression::Null { .. });
        let initializer_is_nullable = self.expression_is_nullable(&typed_expression);
        if initializer_is_null && !nullable {
            let Some(expected_name) = type_name else {
                return Err(CompileError::new(
                    expression.span(),
                    "cannot infer a binding type from `null`; add a nullable type annotation",
                ));
            };
            return Err(CompileError::new(
                expression.span(),
                format!("cannot assign null to non-nullable type `{expected_name}`"),
            ));
        }
        if initializer_is_nullable && !nullable {
            return Err(CompileError::new(
                expression.span(),
                format!("nullable value cannot initialize non-nullable binding `{name}`"),
            ));
        }
        if nullable && expected.is_none() {
            return Err(CompileError::new(
                span,
                "nullable bindings require an explicit type annotation",
            ));
        }
        if let Some(expected) = expected
            && !initializer_is_null
            && actual.base != expected
        {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "type mismatch: binding `{name}` expects `{}`, expression produces `{}`",
                    self.registry.type_name(expected),
                    self.registry.value_type_name(actual)
                ),
            ));
        }
        let value_type = if initializer_is_null {
            ValueType::plain(expected.expect("nullable null initializer has an annotation"))
        } else {
            actual
        };
        let variable =
            self.bind_local_variable(name.to_owned(), value_type, mutable, nullable, span);
        Ok(TypedStatement::VariableDeclaration {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            mutable,
            value_type,
            expression: typed_expression,
            span,
        })
    }

    /// Compiles one assignment after resolving and validating its target binding.
    fn compile_assignment(
        &mut self,
        name: &str,
        expression: &Expression,
        span: syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        let variable = self
            .resolve_variable(name)
            .ok_or_else(|| CompileError::new(span, format!("unknown binding `{name}`")))?;
        if !variable.mutable {
            return Err(CompileError::new(
                span,
                format!("cannot assign to immutable binding `{name}`"),
            ));
        }
        let typed_expression =
            self.compile_expression(expression, Some(variable.value_type.base))?;
        let actual = typed_expression.output.ok_or_else(|| {
            CompileError::new(expression.span(), "a void expression cannot be assigned")
        })?;
        let assigned_null = matches!(expression, Expression::Null { .. });
        let assigned_nullable = self.expression_is_nullable(&typed_expression);
        if assigned_null && !variable.nullable {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "cannot assign null to non-nullable type `{}`",
                    self.registry.value_type_name(variable.value_type)
                ),
            ));
        }
        if assigned_nullable && !variable.nullable {
            return Err(CompileError::new(
                expression.span(),
                format!("nullable value cannot be assigned to binding `{name}`"),
            ));
        }
        if !assigned_null && actual != variable.value_type {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "type mismatch: binding `{name}` expects `{}`, expression produces `{}`",
                    self.registry.value_type_name(variable.value_type),
                    self.registry.value_type_name(actual)
                ),
            ));
        }
        if variable.nullable {
            self.narrowed_bindings.remove(&variable.binding);
        }
        Ok(TypedStatement::Assignment {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            expression: typed_expression,
            span,
        })
    }

    /// Compiles a source block inside a fresh lexical variable scope.
    fn compile_block(&mut self, block: &Block) -> Result<TypedBlock, CompileError> {
        self.variable_scopes.push(HashMap::new());
        self.import_scopes.push(ImportScope::default());
        let result = (|| {
            let mut statements = Vec::with_capacity(block.statements.len());
            for statement in &block.statements {
                if matches!(statement, Statement::Configuration { .. }) {
                    return Err(CompileError::new(
                        statement_span(statement),
                        "`@config` is only allowed at the root level",
                    ));
                }
                if let Statement::Use(declaration) = statement {
                    self.compile_use_declaration(declaration)?;
                    continue;
                }
                statements.push(self.compile_statement(statement)?);
            }
            Ok(TypedBlock {
                statements,
                span: block.span,
            })
        })();
        self.import_scopes.pop();
        self.variable_scopes.pop();
        result
    }

    /// Compiles a `for (variable in start..end)` integer range loop.
    ///
    /// Both bounds compile as ordinary expressions so variables and arithmetic
    /// are accepted. Each bound must be a plain integer; compatibility between
    /// the two bounds is validated through the registry's comparison contract.
    /// The loop variable takes the start bound's type and lives in a dedicated
    /// child scope, where it may shadow an outer binding and is released with
    /// the loop.
    fn compile_for_statement(
        &mut self,
        variable: &str,
        start: &Expression,
        end: &Expression,
        body: &Block,
        span: syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        let typed_start = self.compile_expression(start, None)?;
        let start_type = typed_start.output.ok_or_else(|| {
            CompileError::new(start.span(), "a void expression cannot bound a for range")
        })?;
        self.require_plain_integer_bound(start_type, start.span())?;
        let typed_end = self.compile_expression(end, None)?;
        let end_type = typed_end.output.ok_or_else(|| {
            CompileError::new(end.span(), "a void expression cannot bound a for range")
        })?;
        self.require_plain_integer_bound(end_type, end.span())?;
        let comparison = self
            .registry
            .resolve_comparison_operation(CoreComparisonOperator::Less, start_type, end_type)
            .map_err(|error| CompileError::core(span, error))?;
        let increment_unit = self
            .registry
            .parse_numeric("1", Some(start_type.base))
            .map_err(|error| CompileError::core(start.span(), error))?;
        let increment = self
            .registry
            .resolve_binary_operation(
                CoreBinaryOperator::Addition,
                start_type,
                increment_unit.value_type(),
            )
            .map_err(|error| CompileError::core(span, error))?;
        self.require_plain_integer_bound(increment.output, span)?;
        self.variable_scopes.push(HashMap::new());
        self.import_scopes.push(ImportScope::default());
        let compiled = (|| {
            let local_variable =
                self.bind_local_variable(variable.to_owned(), start_type, false, false, span);
            self.loop_depth += 1;
            let compiled_body = self.compile_block(body);
            self.loop_depth -= 1;
            Ok(TypedStatement::For {
                variable: variable.to_owned(),
                binding: local_variable.binding,
                slot: local_variable.slot,
                variable_type: start_type,
                start: typed_start,
                end: typed_end,
                range_plan: TypedRangePlan {
                    current_type: start_type,
                    increment_unit,
                    comparison,
                    increment,
                },
                body: compiled_body?,
                span,
            })
        })();
        self.import_scopes.pop();
        self.variable_scopes.pop();
        compiled
    }

    /// Validates that a range bound is a plain integer type.
    ///
    /// The type extension explicitly declares whether its base representation
    /// is integral. Subtype-qualified magnitudes are rejected because loop
    /// comparison and increment operate on plain values.
    fn require_plain_integer_bound(
        &self,
        bound: ValueType,
        span: syntax::Span,
    ) -> Result<(), CompileError> {
        if bound.subtype.is_some() {
            return Err(CompileError::new(
                span,
                format!(
                    "for range bounds must be integers, found `{}`",
                    self.registry.value_type_name(bound)
                ),
            ));
        }
        if self
            .registry
            .is_integer_type(bound.base)
            .map_err(|error| CompileError::core(span, error))?
        {
            Ok(())
        } else {
            Err(CompileError::new(
                span,
                format!(
                    "for range bounds must be integers, found `{}`",
                    self.registry.value_type_name(bound)
                ),
            ))
        }
    }
}

#[cfg(test)]
#[path = "compiler.tests.rs"]
mod tests;
