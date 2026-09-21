//! Declared and dynamic array literals.

use crate::ir::{TypedExpression, TypedExpressionKind};
use crate::semantic::{ArrayType, SemanticType};
use crate::syntax::Expression;

use crate::CompileError;

use crate::compiler::Compiler;

impl Compiler<'_> {
    /// Compiles one array literal against a declared or dynamic element contract.
    ///
    /// A declared contract constrains every element through
    /// [`Compiler::compile_array_element`]. Without a declaration the element
    /// declaration creates a dynamic array whose elements keep their concrete
    /// runtime identities. Empty dynamic literals therefore need no inferred
    /// placeholder type.
    pub(crate) fn compile_array_expression(
        &mut self,
        expression: &Expression,
        declared: Option<ArrayType>,
    ) -> Result<TypedExpression, CompileError> {
        let Expression::ArrayLiteral { elements, span } = expression else {
            return Err(CompileError::new(
                expression.span(),
                "an array binding must be initialized with an array literal",
            ));
        };
        let mut typed_elements = Vec::with_capacity(elements.len());
        let array_type = match declared {
            Some(array_type) => {
                for element in elements {
                    typed_elements.push(self.compile_array_element(element, array_type.clone())?);
                }
                array_type
            }
            None => {
                for element in elements {
                    let typed = match self.compile_expression(element, None) {
                        Ok(typed) => typed,
                        Err(error) => {
                            let Some(default_integer) = self.registry.default_integer().ok() else {
                                return Err(error);
                            };
                            if !Self::is_integer_literal_range_error(&error) {
                                return Err(error);
                            }
                            match self.compile_widened_integer_literal(element, default_integer)? {
                                Some(typed) => typed,
                                None => return Err(error),
                            }
                        }
                    };
                    typed.output.as_ref().ok_or_else(|| {
                        CompileError::new(element.span(), "an array element must produce a value")
                    })?;
                    typed_elements.push(typed);
                }
                ArrayType::dynamic()
            }
        };
        // A dynamic contract makes this map a no-op; a static contract prepares
        // every element before the payload exists.
        let elements = typed_elements
            .into_iter()
            .map(|element| self.prepare_element_for_storage(element, array_type.clone()))
            .collect::<Result<Vec<_>, CompileError>>()?;
        Ok(TypedExpression {
            output: Some(SemanticType::Array(std::sync::Arc::new(array_type))),
            kind: TypedExpressionKind::ArrayLiteral { elements },
            span: *span,
        })
    }
}
