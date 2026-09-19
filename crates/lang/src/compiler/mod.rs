//! The source-to-IR driver.
//!
//! This module owns the compiler state and walks the source program, delegating
//! one concern per child module: imports, configuration, lexical scopes,
//! expressions, and the small conversion helpers they share.

use std::collections::HashMap;
use std::sync::Arc;

use crate::semantic::{
    BinaryOperator as CoreBinaryOperator, ComparisonOperator as CoreComparisonOperator, Registry,
    SemanticType, ValueType,
};
use crate::syntax::{Block, Expression, Program, Statement, TypeExpression};

use crate::containers::array::compiler::ArrayElementFlow;
use crate::ir::{
    BindingId, BindingMetadata, CompleteTypeDomain, LocalVariableSlot, TypedBlock, TypedExpression,
    TypedProgram, TypedRangePlan, TypedStatement,
};

mod configuration;
mod dynamic;
mod error;
mod expressions;
mod helpers;
mod imports;
mod scopes;

use self::helpers::statement_span;

pub use error::CompileError;

pub fn compile(program: &Program, registry: &Registry) -> Result<TypedProgram, CompileError> {
    Compiler {
        registry,
        variable_scopes: vec![HashMap::new()],
        scope_slots: vec![Vec::new()],
        next_local_slot: 0,
        bindings: Vec::new(),
        narrowed_bindings: HashMap::new(),
        element_flow: ArrayElementFlow::new(),
        loop_depth: 0,
        loop_contexts: Vec::new(),
        import_scopes: vec![ImportScope::default()],
    }
    .compile_program(program)
}

/// Bounds the loop fixed-point search.
///
/// A pass over a loop body only ever removes facts from the loop head, and the
/// head is a finite set of element slots, so the analysis converges in fewer
/// passes than it has facts. The bound is a defensive backstop for a future
/// change that could reintroduce a fact: it turns a hypothetical compile-time
/// hang into a conservatively merged loop state.
const MAXIMUM_LOOP_FIXED_POINT_PASSES: usize = 64;

pub(crate) struct Compiler<'a> {
    pub(crate) registry: &'a Registry,
    variable_scopes: Vec<HashMap<String, LocalVariable>>,
    scope_slots: Vec<Vec<LocalVariableSlot>>,
    next_local_slot: usize,
    bindings: Vec<BindingMetadata>,
    narrowed_bindings: HashMap<BindingId, usize>,
    /// Records the static type of each element of a literal-initialized array.
    ///
    /// An unconstrained array keeps no element subtype in its contract, so a
    /// constant element read such as `sizes[0]` would otherwise lose the stored
    /// element's subtype. Recording the literal element types preserves that
    /// subtype for constant reads, and a constant write updates one entry while
    /// a dynamic write removes the record and falls back to the array contract.
    ///
    /// A slot holds `Some` for an element whose complete type the compiler
    /// knows and `None` for one whose subtype is only known at runtime, so a
    /// read can tell a precise element type from a dynamic one.
    pub(crate) element_flow: ArrayElementFlow,
    loop_depth: usize,
    /// One entry per enclosing loop whose statements are currently compiled.
    ///
    /// A loop records the flow state of every reachable `break` here, because
    /// the statements after such a `break` are unreachable and must not
    /// contribute to the state an exit propagates.
    loop_contexts: Vec<LoopContext>,
    import_scopes: Vec<ImportScope>,
}

/// Captures the control-flow facts one enclosing loop needs while its body is
/// compiled.
struct LoopContext {
    /// Element type snapshots taken at each reachable `break` in the loop body.
    ///
    /// Every pass over the body replaces this list, so a fixed-point pass that
    /// discards stale facts never leaves an exit recorded from an earlier and
    /// more precise pass behind.
    break_exits: Vec<ArrayElementFlow>,
}

/// Records one compiled pass over a loop body.
///
/// A fixed-point search compiles several passes and keeps only the converged
/// one, so each pass reports everything the loop statement needs from it.
struct CompiledLoopPass {
    /// The pass's compiled condition, which only a `while` loop produces.
    condition: Option<TypedExpression>,
    /// The pass's compiled body.
    body: TypedBlock,
}

/// Records the converged fixed point of one loop.
struct CompiledLoop {
    /// The converged compiled pass.
    pass: CompiledLoopPass,
    /// The element flow state every reachable loop exit agrees on.
    exit_state: ArrayElementFlow,
}

/// Stores the semantic type and statically allocated storage of one local variable.
#[derive(Clone)]
pub(crate) struct LocalVariable {
    pub(crate) binding: BindingId,
    pub(crate) semantic_type: SemanticType,
    pub(crate) slot: LocalVariableSlot,
    pub(crate) mutable: bool,
    pub(crate) nullable: bool,
    pub(crate) complete_type_domain: Option<Arc<CompleteTypeDomain>>,
    pub(crate) adaptive_integer: bool,
}

/// Stores the semantic contract resolved from one explicit binding annotation.
#[derive(Clone, Copy)]
struct ResolvedBindingType {
    semantic_type: SemanticType,
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
            } => {
                let type_expression = TypeExpression::Named {
                    name: type_name.clone(),
                    span: *span,
                };
                self.compile_binding_declaration(
                    name,
                    Some(&type_expression),
                    expression,
                    false,
                    *span,
                )
            }
            Statement::BindingDeclaration {
                kind,
                name,
                type_expression,
                expression,
                span,
            } => self.compile_binding_declaration(
                name,
                type_expression.as_ref(),
                expression,
                matches!(kind, crate::syntax::BindingKind::Let),
                *span,
            ),
            Statement::Assignment {
                name,
                expression,
                span,
            } => self.compile_assignment(name, expression, *span),
            Statement::IndexedAssignment {
                name,
                index,
                expression,
                span,
            } => self.compile_indexed_assignment(name, index, expression, *span),
            Statement::Block(block) => Ok(TypedStatement::Block(self.compile_block(block)?)),
            Statement::If {
                condition,
                body,
                else_body,
                span,
            } => {
                let typed_condition = self.compile_boolean_condition(condition, "if")?;
                let before = self.element_flow.snapshot();
                let narrowed_binding = self.non_null_narrowing_binding(condition);
                if let Some(binding) = narrowed_binding {
                    self.push_narrowing(binding);
                }
                let compiled_body = self.compile_block(body);
                if let Some(binding) = narrowed_binding {
                    self.pop_narrowing(binding);
                }
                let compiled_body = compiled_body?;
                let then = self.element_flow.snapshot();
                self.element_flow.restore(before.clone());
                let compiled_else = else_body
                    .as_ref()
                    .map(|body| self.compile_block(body))
                    .transpose()?;
                let otherwise = self.element_flow.snapshot();
                self.element_flow.merge(&[then, otherwise]);
                Ok(TypedStatement::If {
                    condition: typed_condition,
                    body: compiled_body,
                    else_body: compiled_else,
                    span: *span,
                })
            }
            Statement::While {
                condition,
                body,
                span,
            } => self.compile_while_statement(condition, body, *span),
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
                // The exit state is the state the jump leaves behind, so it is
                // captured here rather than read from the end of the loop body:
                // any statement written after this `break` is unreachable and
                // must not reach the loop's post state.
                self.record_loop_break_exit();
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
        self.require_scalar_expression(&typed_condition)?;
        let expected = ValueType::plain(
            self.registry
                .default_boolean()
                .map_err(|error| CompileError::core(condition.span(), error))?,
        );
        let actual = match typed_condition.output {
            Some(SemanticType::Scalar(actual)) => actual,
            Some(SemanticType::Array(_)) => {
                unreachable!("require_scalar_expression rejects array semantic types")
            }
            None => {
                return Err(CompileError::new(
                    condition.span(),
                    format!(
                        "{construct_name} condition must produce `{}`, found `void`",
                        self.registry.value_type_name(expected)
                    ),
                ));
            }
        };
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
    /// Resolves one parsed binding annotation without changing its syntax tree.
    fn resolve_binding_type(
        &self,
        type_expression: &TypeExpression,
        span: crate::syntax::Span,
    ) -> Result<ResolvedBindingType, CompileError> {
        match type_expression {
            TypeExpression::Named { name, .. } => {
                let base = self
                    .registry
                    .type_by_name(name)
                    .ok_or_else(|| CompileError::new(span, format!("unknown type `{name}`")))?;
                Ok(ResolvedBindingType {
                    semantic_type: SemanticType::Scalar(ValueType::plain(base)),
                    nullable: false,
                })
            }
            TypeExpression::Qualified { .. } => Err(CompileError::new(
                span,
                format!("unknown type `{type_expression}`"),
            )),
            TypeExpression::Array { element, .. } => Ok(ResolvedBindingType {
                semantic_type: SemanticType::Array(self.resolve_array_type(element, span)?),
                nullable: false,
            }),
            TypeExpression::Nullable { inner, .. } => {
                let mut resolved = self.resolve_binding_type(inner, span)?;
                resolved.nullable = true;
                Ok(resolved)
            }
        }
    }

    /// Reports whether a scalar binding carries adaptive `int` semantics.
    ///
    /// The source spelling matters here: `int` and `int64` resolve to the same
    /// base type, but an inferred array made from an adaptive binding must not
    /// silently turn a fixed-width `int64` source into an adaptive container.
    fn binding_uses_adaptive_integer(
        &self,
        type_expression: Option<&TypeExpression>,
        semantic_type: SemanticType,
    ) -> bool {
        let SemanticType::Scalar(value_type) = semantic_type else {
            return false;
        };
        let Some(default_integer) = self.registry.default_integer().ok() else {
            return false;
        };
        if value_type.base != default_integer {
            return false;
        }
        match type_expression {
            Some(type_expression) => Self::type_expression_is_adaptive_integer(type_expression),
            None => true,
        }
    }

    /// Reports whether an annotation spells the adaptive integer family.
    fn type_expression_is_adaptive_integer(type_expression: &TypeExpression) -> bool {
        match type_expression {
            TypeExpression::Named { name, .. } => name == "int",
            TypeExpression::Nullable { inner, .. } => {
                Self::type_expression_is_adaptive_integer(inner)
            }
            TypeExpression::Qualified { .. } | TypeExpression::Array { .. } => false,
        }
    }

    /// Compiles one source binding after enforcing current-scope declaration rules.
    fn compile_binding_declaration(
        &mut self,
        name: &str,
        type_expression: Option<&TypeExpression>,
        expression: &Expression,
        mutable: bool,
        span: crate::syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        if self.variable_declared_in_current_scope(name) {
            return Err(CompileError::new(
                span,
                format!("binding `{name}` is already declared in this scope"),
            ));
        }
        let resolved_type = type_expression
            .map(|type_expression| self.resolve_binding_type(type_expression, span))
            .transpose()?;
        let nullable = resolved_type.is_some_and(|resolved| resolved.nullable);
        let (array_annotation, expected) =
            match resolved_type.map(|resolved| resolved.semantic_type) {
                Some(SemanticType::Array(array_type)) => (Some(array_type), None),
                Some(SemanticType::Scalar(value_type)) => (None, Some(value_type.base)),
                None => (None, None),
            };
        if array_annotation.is_some() && nullable {
            return Err(CompileError::new(span, "array bindings cannot be nullable"));
        }
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
        let typed_expression = if let Some(array_type) = array_annotation {
            self.compile_array_expression(expression, Some(array_type))?
        } else {
            self.compile_expression(expression, expected)?
        };
        let actual = typed_expression.output.ok_or_else(|| {
            CompileError::new(
                expression.span(),
                "a void expression cannot initialize a binding",
            )
        })?;
        if typed_expression.array_type().is_some()
            && let Some(expected) = expected
        {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "binding `{name}` expects `{}`, but the initializer is an array",
                    self.registry.type_name(expected)
                ),
            ));
        }
        let initializer_is_null = matches!(expression, Expression::Null { .. });
        let initializer_is_nullable = self.expression_is_nullable(&typed_expression);
        if initializer_is_null && !nullable {
            let Some(type_expression) = type_expression else {
                return Err(CompileError::new(
                    expression.span(),
                    "cannot infer a binding type from `null`; add a nullable type annotation",
                ));
            };
            return Err(CompileError::new(
                expression.span(),
                format!("cannot assign null to non-nullable type `{type_expression}`"),
            ));
        }
        if initializer_is_nullable && !nullable && expected.is_some() {
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
        // A declaration without an annotation takes the nullability of its
        // initializer, so `let first = a->shift()` declares the nullable
        // binding the removal's element type promises. An explicit annotation
        // still decides, and a nullable initializer that contradicts a
        // non-nullable annotation is rejected above.
        let nullable = nullable || (initializer_is_nullable && expected.is_none());
        if let Some(expected) = expected
            && !initializer_is_null
            && let SemanticType::Scalar(actual) = actual
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
        let semantic_type = if initializer_is_null {
            SemanticType::Scalar(ValueType::plain(
                expected.expect("nullable null initializer has an annotation"),
            ))
        } else {
            actual
        };
        let adaptive_integer = self.binding_uses_adaptive_integer(type_expression, semantic_type);
        let complete_type_domain = typed_expression.complete_type_domain();
        let variable = self.bind_local_variable(
            name.to_owned(),
            semantic_type,
            mutable,
            nullable,
            complete_type_domain,
            span,
        );
        if adaptive_integer {
            self.mark_variable_adaptive_integer(name);
        }
        self.record_array_element_types(variable.binding, &typed_expression);
        Ok(TypedStatement::VariableDeclaration {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            mutable,
            semantic_type,
            expression: typed_expression,
            span,
        })
    }

    /// Compiles one assignment after resolving and validating its target binding.
    fn compile_assignment(
        &mut self,
        name: &str,
        expression: &Expression,
        span: crate::syntax::Span,
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
        let value_type = match variable.semantic_type {
            SemanticType::Scalar(value_type) => value_type,
            SemanticType::Array(_) => {
                return Err(CompileError::new(
                    span,
                    "whole-array reassignment is not supported; assign individual elements",
                ));
            }
        };
        let typed_expression = self.compile_expression(expression, Some(value_type.base))?;
        let actual = match typed_expression.output {
            Some(SemanticType::Scalar(actual)) => actual,
            Some(SemanticType::Array(_)) => {
                return Err(CompileError::new(
                    expression.span(),
                    format!("cannot assign an array to binding `{name}`"),
                ));
            }
            None => {
                return Err(CompileError::new(
                    expression.span(),
                    "a void expression cannot be assigned",
                ));
            }
        };
        let assigned_null = matches!(expression, Expression::Null { .. });
        let assigned_nullable = self.expression_is_nullable(&typed_expression);
        if assigned_null && !variable.nullable {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "cannot assign null to non-nullable type `{}`",
                    self.registry.value_type_name(value_type)
                ),
            ));
        }
        if assigned_nullable && !variable.nullable {
            return Err(CompileError::new(
                expression.span(),
                format!("nullable value cannot be assigned to binding `{name}`"),
            ));
        }
        if !assigned_null && actual != value_type {
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "type mismatch: binding `{name}` expects `{}`, expression produces `{}`",
                    self.registry.value_type_name(value_type),
                    self.registry.value_type_name(actual)
                ),
            ));
        }
        if variable.nullable {
            self.narrowed_bindings.remove(&variable.binding);
        }
        if let Some(domain) = typed_expression.complete_type_domain() {
            self.mark_variable_dynamic(name, domain);
        }
        Ok(TypedStatement::Assignment {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            expression: typed_expression,
            span,
        })
    }

    /// Compiles an indexed assignment against an existing mutable array binding.
    fn compile_indexed_assignment(
        &mut self,
        name: &str,
        index: &Expression,
        expression: &Expression,
        span: crate::syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        let variable = self
            .resolve_variable(name)
            .ok_or_else(|| CompileError::new(span, format!("unknown binding `{name}`")))?;
        if !variable.mutable {
            return Err(CompileError::new(
                span,
                format!("cannot assign through immutable binding `{name}`"),
            ));
        }
        let array_type = match variable.semantic_type {
            SemanticType::Array(array_type) => array_type,
            SemanticType::Scalar(_) => {
                return Err(CompileError::new(
                    span,
                    format!("binding `{name}` is not an array"),
                ));
            }
        };
        let (typed_index, constant_index, index_extractor, index_dispatch) =
            self.compile_array_index(index)?;
        // The element crosses into storage here, so its destination contract is
        // applied before the store is planned.
        let element = self.compile_array_element(expression, array_type)?;
        let typed_expression = self.prepare_element_for_storage(element, array_type)?;
        // A written element the compiler cannot type precisely makes that slot
        // dynamic, so later reads of it keep dispatching on the stored subtype.
        let element_slot = if typed_expression.complete_type_domain().is_some() {
            None
        } else {
            match typed_expression.output {
                Some(SemanticType::Scalar(value_type)) => Some(value_type),
                Some(SemanticType::Array(_)) | None => None,
            }
        };
        match constant_index {
            Some(index) => {
                self.update_array_element_type(variable.binding, index, element_slot);
            }
            None => {
                self.element_flow.remove(variable.binding);
            }
        }
        Ok(TypedStatement::IndexedAssignment {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            index: typed_index,
            constant_index,
            index_extractor,
            index_dispatch,
            expression: typed_expression,
            span,
        })
    }

    /// Compiles a source block inside a fresh lexical variable scope.
    ///
    /// A statement that always transfers control ends the reachable fallthrough
    /// of the block. Later statements are still compiled so every diagnostic the
    /// program earns is reported, but the flow facts of that unreachable suffix
    /// are discarded before the block returns: an assignment written after a
    /// `break` must not describe the state the `break` propagates.
    fn compile_block(&mut self, block: &Block) -> Result<TypedBlock, CompileError> {
        self.variable_scopes.push(HashMap::new());
        self.scope_slots.push(Vec::new());
        self.import_scopes.push(ImportScope::default());
        let result = (|| {
            let mut statements = Vec::with_capacity(block.statements.len());
            let mut fallthrough = true;
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
                if !fallthrough {
                    // The statement is unreachable. Compile it for diagnostics,
                    // then restore the state the last reachable transfer left so
                    // its facts cannot reach the block's exit state.
                    let reachable_types = self.element_flow.snapshot();
                    let reachable_narrowings = self.narrowed_bindings.clone();
                    statements.push(self.compile_statement(statement)?);
                    self.element_flow.restore(reachable_types);
                    self.narrowed_bindings = reachable_narrowings;
                    continue;
                }
                let typed_statement = self.compile_statement(statement)?;
                let terminates = matches!(
                    typed_statement,
                    TypedStatement::Break { .. } | TypedStatement::Continue { .. }
                );
                statements.push(typed_statement);
                if terminates {
                    fallthrough = false;
                }
            }
            Ok((statements, block.span))
        })();
        let owned_slots = self
            .scope_slots
            .last()
            .expect("compiler always has a scope slot frame")
            .clone();
        self.import_scopes.pop();
        self.scope_slots.pop();
        self.variable_scopes.pop();
        result.map(|(statements, span)| TypedBlock {
            statements,
            owned_slots: owned_slots.into_boxed_slice(),
            span,
        })
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
        span: crate::syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        let typed_start = self.compile_expression(start, None)?;
        if typed_start.array_type().is_some() {
            return Err(CompileError::new(
                start.span(),
                "a for range bound must be an integer, found an array",
            ));
        }
        let start_type = self
            .scalar_expression_type(&typed_start, "a void expression cannot bound a for range")?;
        self.require_plain_integer_bound(start_type, start.span())?;
        let typed_end = self.compile_expression(end, None)?;
        if typed_end.array_type().is_some() {
            return Err(CompileError::new(
                end.span(),
                "a for range bound must be an integer, found an array",
            ));
        }
        let end_type =
            self.scalar_expression_type(&typed_end, "a void expression cannot bound a for range")?;
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
        let range_plan = TypedRangePlan {
            current_type: start_type,
            increment_unit,
            comparison,
            increment,
        };
        let loop_entry_state = self.element_flow.snapshot();
        self.variable_scopes.push(HashMap::new());
        self.scope_slots.push(Vec::new());
        self.import_scopes.push(ImportScope::default());
        let compiled = (|| {
            let local_variable = self.bind_local_variable(
                variable.to_owned(),
                SemanticType::Scalar(start_type),
                false,
                false,
                None,
                span,
            );

            let compiled = self.compile_loop_statement(loop_entry_state, true, |compiler| {
                compiler.loop_depth += 1;
                let compiled_body = compiler.compile_block(body);
                compiler.loop_depth -= 1;
                let compiled_body = compiled_body?;
                Ok(CompiledLoopPass {
                    condition: None,
                    body: compiled_body,
                })
            })?;
            Ok(TypedStatement::For {
                variable: variable.to_owned(),
                binding: local_variable.binding,
                slot: local_variable.slot,
                variable_type: start_type,
                start: typed_start,
                end: typed_end,
                range_plan,
                body: compiled.pass.body,
                span,
            })
        })();
        self.import_scopes.pop();
        self.scope_slots.pop();
        self.variable_scopes.pop();
        compiled
    }

    /// Compiles a `while (condition) { body }` loop.
    ///
    /// The condition is part of the loop, not a one-time guard: it is compiled
    /// against the loop's head state and recompiled while the head is refined,
    /// so a condition that reads a mutated element stops trusting the element's
    /// first-iteration subtype. The loop also falls through when the condition
    /// is false before its first iteration, so the entry state always
    /// contributes to the post-loop state.
    fn compile_while_statement(
        &mut self,
        condition: &Expression,
        body: &Block,
        span: crate::syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        let loop_entry_state = self.element_flow.snapshot();
        let narrowed_binding = self.non_null_narrowing_binding(condition);
        if let Some(binding) = narrowed_binding {
            self.push_narrowing(binding);
        }
        let compiled = self.compile_loop_statement(loop_entry_state, true, |compiler| {
            // The condition is compiled on every pass so it observes the current
            // head, and the converged pass is the one that is emitted.
            let typed_condition = compiler.compile_boolean_condition(condition, "while")?;
            compiler.loop_depth += 1;
            let compiled_body = compiler.compile_block(body);
            compiler.loop_depth -= 1;
            let compiled_body = compiled_body?;
            Ok(CompiledLoopPass {
                condition: Some(typed_condition),
                body: compiled_body,
            })
        });
        if let Some(binding) = narrowed_binding {
            self.pop_narrowing(binding);
        }
        let compiled = compiled?;
        let typed_condition = compiled
            .pass
            .condition
            .ok_or_else(|| CompileError::new(span, "`while` requires a compiled loop condition"))?;
        Ok(TypedStatement::While {
            condition: typed_condition,
            body: compiled.pass.body,
            span,
        })
    }

    /// Computes the loop head and the final body of one loop by fixed point.
    ///
    /// A loop body can execute repeatedly, so every operation inside it must be
    /// compiled against the element types that hold on *any* iteration. The head
    /// therefore starts at the loop entry state and is refined by joining in the
    /// state each pass reaches, until the join adds nothing and the head stops
    /// changing. The condition and body of the converged pass are the ones that
    /// are emitted, so no operation in the loop was specialized with a fact a
    /// previous iteration can invalidate.
    ///
    /// `body_step` performs one pass with the compiler flow state already set to
    /// the current loop head, records the state of every reachable `break` in the
    /// active [`LoopContext`], and returns the compiled pass. Element writes
    /// inside the pass update the flow state directly, so the state left when the
    /// pass returns is the pass's backedge state.
    ///
    /// `include_entry_state_in_exit` reports whether the loop can leave without
    /// transferring control out of it, which is a `while` loop whose condition
    /// may be false and a `for` loop whose range may be empty. Its exit state
    /// then joins the state the loop started in with the states the loop's own
    /// exits reach.
    ///
    /// A statement written after an unconditional `break` or `continue` is
    /// unreachable and already excluded from the state recorded here, because
    /// [`Compiler::compile_block`] discards the flow facts of an unreachable
    /// suffix.
    fn compile_loop_statement(
        &mut self,
        loop_entry_state: ArrayElementFlow,
        include_entry_state_in_exit: bool,
        mut body_step: impl FnMut(&mut Self) -> Result<CompiledLoopPass, CompileError>,
    ) -> Result<CompiledLoop, CompileError> {
        let mut head = loop_entry_state.clone();
        let mut converged: Option<CompiledLoop> = None;
        for _ in 0..MAXIMUM_LOOP_FIXED_POINT_PASSES {
            // The head is the state every iteration begins from: the loop entry
            // and every path that reached the end of a previous body.
            self.element_flow.restore(head.clone());
            self.loop_contexts.push(LoopContext {
                break_exits: Vec::new(),
            });
            let body_result = body_step(self);
            let context = self
                .loop_contexts
                .pop()
                .expect("the loop context was pushed above");
            let pass = body_result?;
            let backedge_state = self.element_flow.snapshot();
            // Every later iteration begins from a state this pass can reach at
            // the end of the body, either by completing it or by `break`ing out.
            // Joining those into the current head heads the next iteration, and
            // the head is stable once the join adds nothing.
            let mut reached = vec![backedge_state.clone()];
            reached.extend(context.break_exits.iter().cloned());
            let mut head_snapshots = vec![head.clone()];
            head_snapshots.extend(reached);
            let next_head = ArrayElementFlow::join(&head_snapshots);
            // The post-loop state merges only reachable exits. A `continue` is a
            // backedge and contributes through the post-body state; a `break`
            // contributes the state it recorded at the jump. The head already
            // joins every state a previous iteration reached, so it covers each
            // exiting iteration without re-listing them here.
            let mut exit_snapshots = vec![head.clone(), backedge_state];
            exit_snapshots.extend(context.break_exits.iter().cloned());
            if include_entry_state_in_exit {
                exit_snapshots.push(loop_entry_state.clone());
            }
            let exit_state = ArrayElementFlow::join(&exit_snapshots);
            if next_head == head {
                converged = Some(CompiledLoop { pass, exit_state });
                break;
            }
            head = next_head;
        }
        let compiled = converged.ok_or_else(|| {
            // The bound was reached without converging. Dropping every element
            // fact keeps the diagnostic sound instead of emitting operations
            // compiled against a head a later iteration could invalidate.
            self.element_flow.clear();
            CompileError::new(
                crate::syntax::Span { start: 0, end: 0 },
                "the loop flow analysis did not converge",
            )
        })?;
        // Statements after the loop see the merged state of its reachable exits.
        self.element_flow.restore(compiled.exit_state.clone());
        Ok(compiled)
    }

    /// Records the current element flow state as the exit of the innermost loop.
    ///
    /// The state is captured at the `break` rather than read from the end of the
    /// loop body, so a statement written after the jump cannot reach the loop's
    /// post state. The innermost enclosing [`LoopContext`] always exists here,
    /// because `break` is rejected outside a loop.
    fn record_loop_break_exit(&mut self) {
        let Some(context) = self.loop_contexts.last_mut() else {
            return;
        };
        context.break_exits.push(self.element_flow.snapshot());
    }

    /// Validates that a range bound is a plain integer type.
    ///
    /// The type extension explicitly declares whether its base representation
    /// is integral. Subtype-qualified magnitudes are rejected because loop
    /// comparison and increment operate on plain values.
    fn require_plain_integer_bound(
        &self,
        bound: ValueType,
        span: crate::syntax::Span,
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
#[path = "mod.tests.rs"]
mod tests;
