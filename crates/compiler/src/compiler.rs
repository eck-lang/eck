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
    ArrayType, BindingId, BindingMetadata, LocalVariableSlot, TypedBlock, TypedExpression,
    TypedProgram, TypedRangePlan, TypedStatement,
};

mod arrays;
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
        array_element_types: HashMap::new(),
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

struct Compiler<'a> {
    registry: &'a Registry,
    variable_scopes: Vec<HashMap<String, LocalVariable>>,
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
    array_element_types: HashMap<BindingId, Vec<Option<ValueType>>>,
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
    break_exits: Vec<HashMap<BindingId, Vec<Option<ValueType>>>>,
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
    exit_state: HashMap<BindingId, Vec<Option<ValueType>>>,
}

/// Stores the semantic type and statically allocated storage of one local variable.
#[derive(Clone, Copy)]
struct LocalVariable {
    binding: BindingId,
    value_type: ValueType,
    slot: LocalVariableSlot,
    mutable: bool,
    nullable: bool,
    array_type: Option<ArrayType>,
    dynamic_complete_type: bool,
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
                let before = self.array_element_type_snapshot();
                let narrowed_binding = self.non_null_narrowing_binding(condition);
                if let Some(binding) = narrowed_binding {
                    self.push_narrowing(binding);
                }
                let compiled_body = self.compile_block(body);
                if let Some(binding) = narrowed_binding {
                    self.pop_narrowing(binding);
                }
                let compiled_body = compiled_body?;
                let then = self.array_element_type_snapshot();
                self.restore_array_element_type_snapshot(before.clone());
                let compiled_else = else_body
                    .as_ref()
                    .map(|body| self.compile_block(body))
                    .transpose()?;
                let otherwise = self.array_element_type_snapshot();
                self.merge_array_element_type_snapshots(&[then, otherwise]);
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
        let array_annotation = type_name
            .filter(|type_name| type_name.ends_with("[]"))
            .map(|type_name| self.resolve_array_type(type_name, span))
            .transpose()?;
        if array_annotation.is_some() && nullable {
            return Err(CompileError::new(span, "array bindings cannot be nullable"));
        }
        let expected = type_name
            .filter(|type_name| !type_name.ends_with("[]"))
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
        let array_type = typed_expression.array_type();
        let dynamic_complete_type = typed_expression.dynamic_complete_type();
        let variable = self.bind_local_variable(
            name.to_owned(),
            value_type,
            mutable,
            nullable,
            array_type,
            dynamic_complete_type,
            span,
        );
        self.record_array_element_types(variable.binding, &typed_expression);
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
        if variable.array_type.is_some() {
            return Err(CompileError::new(
                span,
                "whole-array reassignment is not supported; assign individual elements",
            ));
        }
        let typed_expression =
            self.compile_expression(expression, Some(variable.value_type.base))?;
        let actual = typed_expression.output.ok_or_else(|| {
            CompileError::new(expression.span(), "a void expression cannot be assigned")
        })?;
        if typed_expression.array_type().is_some() {
            return Err(CompileError::new(
                expression.span(),
                format!("cannot assign an array to binding `{name}`"),
            ));
        }
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
        if typed_expression.dynamic_complete_type() {
            self.mark_variable_dynamic(name);
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
        span: syntax::Span,
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
        let array_type = variable
            .array_type
            .ok_or_else(|| CompileError::new(span, format!("binding `{name}` is not an array")))?;
        let (typed_index, constant_index, index_extractor) = self.compile_array_index(index)?;
        let typed_expression = self.compile_array_element(expression, array_type)?;
        // A written element the compiler cannot type precisely makes that slot
        // dynamic, so later reads of it keep dispatching on the stored subtype.
        let element_slot = if typed_expression.dynamic_complete_type() {
            None
        } else {
            typed_expression.output
        };
        match constant_index {
            Some(index) => {
                self.update_array_element_type(variable.binding, index, element_slot);
            }
            None => {
                self.array_element_types.remove(&variable.binding);
            }
        }
        Ok(TypedStatement::IndexedAssignment {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            index: typed_index,
            constant_index,
            index_extractor,
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
                    let reachable_types = self.array_element_type_snapshot();
                    let reachable_narrowings = self.narrowed_bindings.clone();
                    statements.push(self.compile_statement(statement)?);
                    self.restore_array_element_type_snapshot(reachable_types);
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
        if typed_start.array_type().is_some() {
            return Err(CompileError::new(
                start.span(),
                "a for range bound must be an integer, found an array",
            ));
        }
        let start_type = typed_start.output.ok_or_else(|| {
            CompileError::new(start.span(), "a void expression cannot bound a for range")
        })?;
        self.require_plain_integer_bound(start_type, start.span())?;
        let typed_end = self.compile_expression(end, None)?;
        if typed_end.array_type().is_some() {
            return Err(CompileError::new(
                end.span(),
                "a for range bound must be an integer, found an array",
            ));
        }
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
        let range_plan = TypedRangePlan {
            current_type: start_type,
            increment_unit,
            comparison,
            increment,
        };
        let loop_entry_state = self.array_element_type_snapshot();
        self.variable_scopes.push(HashMap::new());
        self.import_scopes.push(ImportScope::default());
        let compiled = (|| {
            let local_variable = self.bind_local_variable(
                variable.to_owned(),
                start_type,
                false,
                false,
                None,
                false,
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
        span: syntax::Span,
    ) -> Result<TypedStatement, CompileError> {
        let loop_entry_state = self.array_element_type_snapshot();
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
        loop_entry_state: HashMap<BindingId, Vec<Option<ValueType>>>,
        include_entry_state_in_exit: bool,
        mut body_step: impl FnMut(&mut Self) -> Result<CompiledLoopPass, CompileError>,
    ) -> Result<CompiledLoop, CompileError> {
        let mut head = loop_entry_state.clone();
        let mut converged: Option<CompiledLoop> = None;
        for _ in 0..MAXIMUM_LOOP_FIXED_POINT_PASSES {
            // The head is the state every iteration begins from: the loop entry
            // and every path that reached the end of a previous body.
            self.restore_array_element_type_snapshot(head.clone());
            self.loop_contexts.push(LoopContext {
                break_exits: Vec::new(),
            });
            let body_result = body_step(self);
            let context = self
                .loop_contexts
                .pop()
                .expect("the loop context was pushed above");
            let pass = body_result?;
            let backedge_state = self.array_element_type_snapshot();
            // Every later iteration begins from a state this pass can reach at
            // the end of the body, either by completing it or by `break`ing out.
            // Joining those into the current head heads the next iteration, and
            // the head is stable once the join adds nothing.
            let mut reached = vec![backedge_state.clone()];
            reached.extend(context.break_exits.iter().cloned());
            let mut head_snapshots = vec![head.clone()];
            head_snapshots.extend(reached);
            let next_head = Self::join_array_element_type_snapshots(&head_snapshots);
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
            let exit_state = Self::join_array_element_type_snapshots(&exit_snapshots);
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
            self.array_element_types.clear();
            CompileError::new(
                syntax::Span { start: 0, end: 0 },
                "the loop flow analysis did not converge",
            )
        })?;
        // Statements after the loop see the merged state of its reachable exits.
        self.restore_array_element_type_snapshot(compiled.exit_state.clone());
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
        context.break_exits.push(self.array_element_types.clone());
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
