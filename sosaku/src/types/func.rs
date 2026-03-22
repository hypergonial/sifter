use crate::types::exp::Exp;

/// Represents a function item in the AST, which consists of a function name and a list of argument expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionItem<'a> {
    name: String,
    args: Vec<Exp<'a>>,
}

impl<'a> FunctionItem<'a> {
    /// Create a new [`FunctionItem`] with the given function name and argument expressions.
    ///
    /// # Parameters
    /// - `name`: The name of the function being called.
    /// - `args`: A vector of `Exp` representing the arguments passed to the function.
    ///
    /// # Returns
    ///
    /// - A new `FunctionItem` instance containing the provided function name and arguments.
    pub fn new(name: impl Into<String>, args: impl Into<Vec<Exp<'a>>>) -> Self {
        Self {
            name: name.into(),
            args: args.into(),
        }
    }

    /// The name of the function.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The argument expressions passed to the function.
    pub fn args(&self) -> &[Exp<'_>] {
        &self.args
    }
}
