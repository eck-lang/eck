//! The source-to-IR driver.
//!
//! This module owns the compiler state and walks the source program, delegating
//! one concern per child module: imports, configuration, lexical scopes,
//! expressions, and the small conversion helpers they share.

use std::collections::HashMap;
use std::sync::Arc;

use crate::semantic::{
    BinaryOperator as CoreBinaryOperator, ComparisonOperator as CoreComparisonOperator,
    DeclaredType, Registry, ScalarRepresentation, SemanticType, ValueType,
};
use crate::syntax::{Block, Expression, Program, Statement, TypeExpression};

use crate::containers::array::compiler::ArrayElementFlow;
use crate::ir::{
    BindingContract, BindingId, BindingMetadata, CompleteTypeDomain, LocalVariableSlot, TypedBlock,
    TypedExpression, TypedProgram, TypedRangePlan, TypedStatement,
};

mod configuration;
mod error;
mod expressions;
mod finite_dispatch;
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
        binding_flow: BindingFlowState::default(),
        element_flow: ArrayElementFlow::new(),
        loop_depth: 0,
        loop_contexts: Vec::new(),
        import_scopes: vec![ImportScope::default()],
        type_aliases: HashMap::new(),
        alias_cache: HashMap::new(),
        alias_resolution_stack: Vec::new(),
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
    binding_flow: BindingFlowState,
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
    type_aliases: HashMap<String, TypeExpression>,
    alias_cache: HashMap<String, DeclaredType>,
    alias_resolution_stack: Vec<String>,
}

/// Captures the control-flow facts one enclosing loop needs while its body is
/// compiled.
struct LoopContext {
    /// Element type snapshots taken at each reachable `break` in the loop body.
    ///
    /// Every pass over the body replaces this list, so a fixed-point pass that
    /// discards stale facts never leaves an exit recorded from an earlier and
    /// more precise pass behind.
    break_exits: Vec<CompilerFlowState>,
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
    exit_state: CompilerFlowState,
}

/// Flow-sensitive knowledge kept independently from binding contracts.
#[derive(Clone, Default, PartialEq)]
struct BindingFlowState {
    current_types: HashMap<BindingId, SemanticType>,
}

impl BindingFlowState {
    /// Joins the current types reachable through every input path.
    fn join(states: &[Self]) -> Self {
        let Some(first) = states.first() else {
            return Self::default();
        };
        let mut current_types = HashMap::new();
        for binding in first.current_types.keys() {
            let members = states
                .iter()
                .filter_map(|state| state.current_types.get(binding).cloned())
                .collect::<Vec<_>>();
            if members.len() == states.len() {
                current_types.insert(*binding, SemanticType::union(members));
            }
        }
        Self { current_types }
    }
}

/// One snapshot of every domain-owned flow analysis.
#[derive(Clone, PartialEq)]
struct CompilerFlowState {
    arrays: ArrayElementFlow,
    bindings: BindingFlowState,
}

/// Stores the semantic type and statically allocated storage of one local variable.
#[derive(Clone)]
pub(crate) struct LocalVariable {
    pub(crate) binding: BindingId,
    pub(crate) semantic_type: SemanticType,
    pub(crate) slot: LocalVariableSlot,
    pub(crate) mutable: bool,
    pub(crate) contract: BindingContract,
    pub(crate) complete_type_domain: Option<Arc<CompleteTypeDomain>>,
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
    /// Captures every domain-owned flow analysis at the current program point.
    fn flow_snapshot(&self) -> CompilerFlowState {
        CompilerFlowState {
            arrays: self.element_flow.snapshot(),
            bindings: self.binding_flow.clone(),
        }
    }

    /// Restores every domain-owned flow analysis to one earlier point.
    fn restore_flow(&mut self, state: CompilerFlowState) {
        self.element_flow.restore(state.arrays);
        self.binding_flow = state.bindings;
    }

    /// Replaces every domain-owned flow analysis with their path join.
    fn merge_flow(&mut self, states: &[CompilerFlowState]) {
        self.restore_flow(Self::join_flow(states));
    }

    /// Joins snapshots without exposing domain internals to control-flow code.
    fn join_flow(states: &[CompilerFlowState]) -> CompilerFlowState {
        CompilerFlowState {
            arrays: ArrayElementFlow::join(
                &states
                    .iter()
                    .map(|state| state.arrays.clone())
                    .collect::<Vec<_>>(),
            ),
            bindings: BindingFlowState::join(
                &states
                    .iter()
                    .map(|state| state.bindings.clone())
                    .collect::<Vec<_>>(),
            ),
        }
    }

    /// Compiles every root statement into a typed program.
    ///
    /// Root-level `use` declarations apply to the root import scope and are
    /// consumed rather than lowered, so they never appear in the typed program.
    /// The result records the final local slot count and binding metadata.
    fn compile_program(&mut self, program: &Program) -> Result<TypedProgram, CompileError> {
        for statement in &program.statements {
            if let Statement::TypeAlias { definition, .. } = statement
                && self
                    .type_aliases
                    .insert(definition.name.clone(), definition.expression.clone())
                    .is_some()
            {
                return Err(CompileError::new(
                    definition.span,
                    format!("type alias `{}` is already declared", definition.name),
                ));
            }
        }
        let mut aliases = self.type_aliases.keys().cloned().collect::<Vec<_>>();
        aliases.sort();
        for alias in aliases {
            let span = self
                .type_aliases
                .get(&alias)
                .map(TypeExpression::span)
                .expect("collected type alias has a source expression");
            self.resolve_declared_type(&TypeExpression::Named { name: alias, span }, span)?;
        }
        let mut statements = Vec::new();
        for statement in &program.statements {
            if let Statement::Use(declaration) = statement {
                self.compile_use_declaration(declaration)?;
                continue;
            }
            if matches!(statement, Statement::TypeAlias { .. }) {
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
            Statement::TypeAlias { .. } => Err(CompileError::new(
                statement_span(statement),
                "type aliases are collected before lowering",
            )),
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
                let before = self.flow_snapshot();
                let narrowed_binding = self.non_null_narrowing_binding(condition);
                if let Some(binding) = narrowed_binding {
                    self.push_narrowing(binding);
                }
                let compiled_body = self.compile_block(body);
                if let Some(binding) = narrowed_binding {
                    self.pop_narrowing(binding);
                }
                let compiled_body = compiled_body?;
                let then = self.flow_snapshot();
                self.restore_flow(before.clone());
                let compiled_else = else_body
                    .as_ref()
                    .map(|body| self.compile_block(body))
                    .transpose()?;
                let otherwise = self.flow_snapshot();
                self.merge_flow(&[then, otherwise]);
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
            Some(SemanticType::Array(_)) | Some(SemanticType::Union(_)) => {
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
    /// Resolves one parsed type expression, including aliases and recursive structure.
    fn resolve_declared_type(
        &mut self,
        type_expression: &TypeExpression,
        span: crate::syntax::Span,
    ) -> Result<DeclaredType, CompileError> {
        match type_expression {
            TypeExpression::Named { name, .. } => {
                if let Some(alias_expression) = self.type_aliases.get(name).cloned() {
                    if let Some(resolved) = self.alias_cache.get(name) {
                        return Ok(resolved.clone());
                    }
                    if let Some(index) = self
                        .alias_resolution_stack
                        .iter()
                        .position(|active| active == name)
                    {
                        let mut cycle = self.alias_resolution_stack[index..].to_vec();
                        cycle.push(name.clone());
                        return Err(CompileError::new(
                            span,
                            format!("type alias cycle: {}", cycle.join(" -> ")),
                        ));
                    }
                    self.alias_resolution_stack.push(name.clone());
                    let result = self.resolve_declared_type(&alias_expression, span);
                    self.alias_resolution_stack.pop();
                    let resolved = result?;
                    self.alias_cache.insert(name.clone(), resolved.clone());
                    return Ok(resolved);
                }
                let base = self
                    .registry
                    .type_by_name(name)
                    .ok_or_else(|| CompileError::new(span, format!("unknown type `{name}`")))?;
                Ok(DeclaredType {
                    semantic_type: SemanticType::Scalar(ValueType::plain(base)),
                    representation: self
                        .registry
                        .type_representation(name)
                        .unwrap_or(ScalarRepresentation::Exact),
                })
            }
            TypeExpression::Qualified { base, subtype, .. } => {
                let mut resolved = self.resolve_declared_type(base, span)?;
                let SemanticType::Scalar(value_type) = resolved.semantic_type else {
                    return Err(CompileError::new(
                        span,
                        "a subtype qualifier requires a scalar type",
                    ));
                };
                let subtype_id = self
                    .registry
                    .subtype_by_suffix(subtype)
                    .or_else(|| self.registry.subtype_by_name(subtype))
                    .ok_or_else(|| {
                        CompileError::new(span, format!("unknown type subtype `{subtype}`"))
                    })?;
                resolved.semantic_type = SemanticType::Scalar(ValueType {
                    base: value_type.base,
                    subtype: Some(subtype_id),
                });
                Ok(resolved)
            }
            TypeExpression::Array { element, .. } => {
                let resolved = self.resolve_declared_type(element, span)?;
                let element_representation = match &resolved.semantic_type {
                    SemanticType::Scalar(_) => resolved.representation,
                    SemanticType::Union(members)
                        if members
                            .iter()
                            .all(|member| matches!(member, SemanticType::Scalar(_))) =>
                    {
                        resolved.representation
                    }
                    SemanticType::Array(_) | SemanticType::Union(_) => ScalarRepresentation::Exact,
                };
                Ok(DeclaredType {
                    semantic_type: SemanticType::Array(Arc::new(
                        crate::semantic::ArrayType::static_element(
                            resolved.semantic_type,
                            element_representation,
                        ),
                    )),
                    representation: ScalarRepresentation::Exact,
                })
            }
            TypeExpression::Nullable { inner, .. } => {
                let resolved = self.resolve_declared_type(inner, span)?;
                let null = self
                    .registry
                    .default_null()
                    .map_err(|error| CompileError::core(span, error))?;
                Ok(DeclaredType {
                    semantic_type: SemanticType::union([
                        resolved.semantic_type,
                        SemanticType::Scalar(ValueType::plain(null)),
                    ]),
                    representation: resolved.representation,
                })
            }
            TypeExpression::Union { members, .. } => {
                let mut representation = ScalarRepresentation::Exact;
                let mut resolved_members = Vec::with_capacity(members.len());
                for member in members {
                    let resolved = self.resolve_declared_type(member, span)?;
                    representation = representation.join(resolved.representation);
                    resolved_members.push(resolved.semantic_type);
                }
                Ok(DeclaredType {
                    semantic_type: SemanticType::union(resolved_members),
                    representation,
                })
            }
        }
    }

    /// Resolves the single array member used to type-check an array literal.
    fn array_member_for_type(semantic_type: &SemanticType) -> Option<crate::semantic::ArrayType> {
        match semantic_type {
            SemanticType::Array(array_type) => Some((**array_type).clone()),
            SemanticType::Union(members) => {
                let mut arrays = members.iter().filter_map(Self::array_member_for_type);
                let first = arrays.next()?;
                arrays.next().is_none().then_some(first)
            }
            _ => None,
        }
    }

    /// Returns every array member that may contextually type one array literal.
    fn array_members_for_type(semantic_type: &SemanticType) -> Vec<crate::semantic::ArrayType> {
        match semantic_type {
            SemanticType::Array(array_type) => vec![(**array_type).clone()],
            SemanticType::Union(members) => members
                .iter()
                .flat_map(Self::array_members_for_type)
                .collect(),
            SemanticType::Scalar(_) => Vec::new(),
        }
    }

    /// Returns the single scalar identity a type can use for literal context.
    fn scalar_member_for_type(semantic_type: &SemanticType) -> Option<ValueType> {
        match semantic_type {
            SemanticType::Scalar(value_type) => Some(*value_type),
            SemanticType::Union(members) => {
                let mut scalars = members.iter().filter_map(Self::scalar_member_for_type);
                let first = scalars.next()?;
                scalars.next().is_none().then_some(first)
            }
            _ => None,
        }
    }

    /// Reports whether a structural type contains an array alternative.
    fn type_contains_array(semantic_type: &SemanticType) -> bool {
        match semantic_type {
            SemanticType::Array(_) => true,
            SemanticType::Scalar(_) => false,
            SemanticType::Union(members) => members.iter().any(Self::type_contains_array),
        }
    }

    /// Renders a structural semantic type with ECK source-level precedence.
    fn semantic_type_name(&self, semantic_type: &SemanticType) -> String {
        match semantic_type {
            SemanticType::Scalar(value_type) => {
                self.registry.value_type_name(*value_type).to_owned()
            }
            SemanticType::Array(array_type) => {
                let Some(element_type) = array_type.static_semantic_type() else {
                    return "dynamic[]".into();
                };
                let element = self.semantic_type_name(element_type);
                match element_type {
                    SemanticType::Union(_) => format!("({element})[]"),
                    _ => format!("{element}[]"),
                }
            }
            SemanticType::Union(members) => members
                .iter()
                .map(|member| self.semantic_type_name(member))
                .collect::<Vec<_>>()
                .join(" | "),
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
            .map(|type_expression| self.resolve_declared_type(type_expression, span))
            .transpose()?;
        let dynamic_binding = mutable && resolved_type.is_none();
        let array_annotation = resolved_type
            .as_ref()
            .and_then(|resolved| Self::array_member_for_type(&resolved.semantic_type));
        let array_annotations = resolved_type
            .as_ref()
            .map(|resolved| Self::array_members_for_type(&resolved.semantic_type))
            .unwrap_or_default();
        let expected = resolved_type
            .as_ref()
            .and_then(|resolved| Self::scalar_member_for_type(&resolved.semantic_type))
            .map(|value_type| value_type.base);
        let mut typed_expression = if let Some(array_type) = array_annotation.clone() {
            if !matches!(expression, Expression::ArrayLiteral { .. }) {
                self.compile_expression(expression, expected)?
            } else {
                self.compile_array_expression(expression, Some(array_type))?
            }
        } else if matches!(expression, Expression::ArrayLiteral { .. })
            && !array_annotations.is_empty()
        {
            let mut last_error = None;
            let mut compiled = None;
            for candidate in &array_annotations {
                match self.compile_array_expression(expression, Some(candidate.clone())) {
                    Ok(expression) => {
                        compiled = Some(expression);
                        break;
                    }
                    Err(error) => last_error = Some(error),
                }
            }
            compiled.ok_or_else(|| {
                last_error.expect("at least one contextual array candidate was compiled")
            })?
        } else {
            self.compile_expression(expression, expected)?
        };
        let mut actual = typed_expression.output.clone().ok_or_else(|| {
            CompileError::new(
                expression.span(),
                "a void expression cannot initialize a binding",
            )
        })?;
        if let Some(destination) = array_annotation
            && matches!(&actual, SemanticType::Array(source) if source.static_semantic_type().is_none())
        {
            if let Some(known_types) = self.array_known_element_types(&typed_expression)
                && let Some(destination_element) = destination.static_semantic_type()
                && known_types
                    .iter()
                    .any(|known| !crate::semantic::is_assignable(known, destination_element))
            {
                return Err(CompileError::new(
                    expression.span(),
                    format!(
                        "dynamic array does not satisfy `{}`",
                        self.semantic_type_name(&SemanticType::array(destination.clone()))
                    ),
                ));
            }
            typed_expression = TypedExpression {
                output: Some(SemanticType::array(destination.clone())),
                kind: crate::ir::TypedExpressionKind::ArrayBoundary {
                    array_type: destination.clone(),
                    expression: Box::new(typed_expression),
                },
                span: expression.span(),
            };
            actual = SemanticType::array(destination);
        }
        if let Some(resolved_type) = &resolved_type {
            let destination_is_nullable =
                self.semantic_type_is_nullable(&resolved_type.semantic_type);
            if self.semantic_type_is_nullable(&actual) && !destination_is_nullable {
                if matches!(expression, Expression::Null { .. }) {
                    return Err(CompileError::new(
                        expression.span(),
                        format!(
                            "cannot assign null to non-nullable type `{}`",
                            type_expression.expect("resolved annotation")
                        ),
                    ));
                }
                return Err(CompileError::new(
                    expression.span(),
                    format!("nullable value cannot initialize non-nullable binding `{name}`"),
                ));
            }
            if matches!(actual, SemanticType::Array(_))
                && !Self::type_contains_array(&resolved_type.semantic_type)
            {
                let expected_name = Self::scalar_member_for_type(&resolved_type.semantic_type)
                    .map(|value_type| self.registry.value_type_name(value_type).to_owned())
                    .unwrap_or_else(|| type_expression.expect("resolved annotation").to_string());
                return Err(CompileError::new(
                    expression.span(),
                    format!(
                        "binding `{name}` expects `{expected_name}`, but the initializer is an array"
                    ),
                ));
            }
            if !crate::semantic::is_assignable(&actual, &resolved_type.semantic_type) {
                let expected_name = self.semantic_type_name(&resolved_type.semantic_type);
                let actual_name = self.semantic_type_name(&actual);
                return Err(CompileError::new(
                    expression.span(),
                    format!(
                        "type mismatch: binding `{name}` expects `{expected_name}`, expression produces `{actual_name}`"
                    ),
                ));
            }
        }
        let semantic_type = match resolved_type {
            Some(resolved) => match (&resolved.semantic_type, &actual) {
                // Preserve a concrete scalar subtype for the existing
                // subtype-aware operator plans. A nullable or union
                // declaration remains its declared structural type so
                // nullability and membership are not erased from reads.
                (SemanticType::Scalar(_), SemanticType::Scalar(actual)) => {
                    SemanticType::Scalar(*actual)
                }
                _ => resolved.semantic_type,
            },
            None => actual,
        };
        let complete_type_domain = typed_expression.complete_type_domain();
        let contract = if dynamic_binding {
            BindingContract::Dynamic
        } else {
            BindingContract::Static(semantic_type.clone())
        };
        let variable = self.bind_local_variable(
            name.to_owned(),
            semantic_type.clone(),
            mutable,
            contract,
            complete_type_domain,
            span,
        );
        self.record_array_element_types(variable.binding, &typed_expression);
        Ok(TypedStatement::VariableDeclaration {
            name: name.to_owned(),
            binding: variable.binding,
            slot: variable.slot,
            mutable,
            semantic_type: semantic_type.clone(),
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
        // A non-null proof narrows reads, not the destination of an assignment:
        // assigning `null` inside `if (value != null)` must be checked against
        // the binding's declared union so the proof can then be invalidated.
        let dynamic_binding = matches!(variable.contract, BindingContract::Dynamic);
        let target_type = match variable.contract {
            BindingContract::Dynamic => self.effective_variable_semantic_type(&variable),
            BindingContract::Static(_) => variable.semantic_type.clone(),
        };
        let value_type = match &target_type {
            SemanticType::Scalar(value_type) => Some(*value_type),
            SemanticType::Array(_) | SemanticType::Union(_)
                if !dynamic_binding && Self::type_contains_array(&target_type) =>
            {
                return Err(CompileError::new(
                    span,
                    "whole-array reassignment is not supported; assign individual elements",
                ));
            }
            _ => None,
        };
        let typed_expression = self.compile_expression(
            expression,
            (!dynamic_binding)
                .then(|| value_type.map(|value| value.base))
                .flatten(),
        )?;
        let actual = typed_expression.output.as_ref().ok_or_else(|| {
            CompileError::new(expression.span(), "a void expression cannot be assigned")
        })?;
        if dynamic_binding {
            self.update_dynamic_binding_type(variable.binding, actual.clone());
            self.record_array_element_types(variable.binding, &typed_expression);
            self.narrowed_bindings.remove(&variable.binding);
            return Ok(TypedStatement::Assignment {
                name: name.to_owned(),
                binding: variable.binding,
                slot: variable.slot,
                expression: typed_expression,
                span,
            });
        }
        let target_is_nullable = self.semantic_type_is_nullable(&target_type);
        if self.semantic_type_is_nullable(actual) && !target_is_nullable {
            if matches!(expression, Expression::Null { .. }) {
                return Err(CompileError::new(
                    expression.span(),
                    format!(
                        "cannot assign null to non-nullable type `{}`",
                        self.registry.value_type_name(
                            Self::scalar_member_for_type(&target_type).ok_or_else(|| {
                                CompileError::new(expression.span(), "binding is not scalar")
                            })?
                        )
                    ),
                ));
            }
            return Err(CompileError::new(
                expression.span(),
                format!("nullable value cannot be assigned to binding `{name}`"),
            ));
        }
        if matches!(actual, SemanticType::Array(_)) && !Self::type_contains_array(&target_type) {
            return Err(CompileError::new(
                expression.span(),
                format!("cannot assign an array to binding `{name}`"),
            ));
        }
        if !crate::semantic::is_assignable(actual, &target_type) {
            let expected_name = self.semantic_type_name(&target_type);
            let actual_name = self.semantic_type_name(actual);
            return Err(CompileError::new(
                expression.span(),
                format!(
                    "type mismatch: binding `{name}` expects `{expected_name}`, expression produces `{actual_name}`"
                ),
            ));
        }
        if self.semantic_type_is_nullable(&variable.semantic_type) {
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
        let array_type = match self.effective_variable_semantic_type(&variable) {
            SemanticType::Array(array_type) => (*array_type).clone(),
            _ => {
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
        let element = self.compile_array_element(expression, array_type.clone())?;
        let typed_expression = self.prepare_element_for_storage(element, array_type.clone())?;
        // A written element the compiler cannot type precisely makes that slot
        // dynamic, so later reads of it keep dispatching on the stored subtype.
        let element_slot = self.array_flow_type_for_expression(&typed_expression);
        match constant_index {
            Some(index) => {
                self.update_array_element_type(variable.binding, index, element_slot);
            }
            None => {
                self.update_array_unknown_index_type(variable.binding, &typed_expression);
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
                    let reachable_types = self.flow_snapshot();
                    let reachable_narrowings = self.narrowed_bindings.clone();
                    statements.push(self.compile_statement(statement)?);
                    self.restore_flow(reachable_types);
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
        let loop_entry_state = self.flow_snapshot();
        self.variable_scopes.push(HashMap::new());
        self.scope_slots.push(Vec::new());
        self.import_scopes.push(ImportScope::default());
        let compiled = (|| {
            let local_variable = self.bind_local_variable(
                variable.to_owned(),
                SemanticType::Scalar(start_type),
                false,
                BindingContract::Static(SemanticType::Scalar(start_type)),
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
        let loop_entry_state = self.flow_snapshot();
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
        loop_entry_state: CompilerFlowState,
        include_entry_state_in_exit: bool,
        mut body_step: impl FnMut(&mut Self) -> Result<CompiledLoopPass, CompileError>,
    ) -> Result<CompiledLoop, CompileError> {
        let mut head = loop_entry_state.clone();
        let mut converged: Option<CompiledLoop> = None;
        for _ in 0..MAXIMUM_LOOP_FIXED_POINT_PASSES {
            // The head is the state every iteration begins from: the loop entry
            // and every path that reached the end of a previous body.
            self.restore_flow(head.clone());
            self.loop_contexts.push(LoopContext {
                break_exits: Vec::new(),
            });
            let body_result = body_step(self);
            let context = self
                .loop_contexts
                .pop()
                .expect("the loop context was pushed above");
            let pass = body_result?;
            let backedge_state = self.flow_snapshot();
            // Every later iteration begins from a state this pass can reach at
            // the end of the body, either by completing it or by `break`ing out.
            // Joining those into the current head heads the next iteration, and
            // the head is stable once the join adds nothing.
            let mut reached = vec![backedge_state.clone()];
            reached.extend(context.break_exits.iter().cloned());
            let mut head_snapshots = vec![head.clone()];
            head_snapshots.extend(reached);
            let next_head = Self::join_flow(&head_snapshots);
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
            let exit_state = Self::join_flow(&exit_snapshots);
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
            self.binding_flow.current_types.clear();
            CompileError::new(
                crate::syntax::Span { start: 0, end: 0 },
                "the loop flow analysis did not converge",
            )
        })?;
        // Statements after the loop see the merged state of its reachable exits.
        self.restore_flow(compiled.exit_state.clone());
        Ok(compiled)
    }

    /// Records the current element flow state as the exit of the innermost loop.
    ///
    /// The state is captured at the `break` rather than read from the end of the
    /// loop body, so a statement written after the jump cannot reach the loop's
    /// post state. The innermost enclosing [`LoopContext`] always exists here,
    /// because `break` is rejected outside a loop.
    fn record_loop_break_exit(&mut self) {
        let snapshot = self.flow_snapshot();
        let Some(context) = self.loop_contexts.last_mut() else {
            return;
        };
        context.break_exits.push(snapshot);
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
