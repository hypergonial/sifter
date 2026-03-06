use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Write},
    sync::Arc,
};

use nom::Finish;
use serde::Deserialize;
use thiserror::Error;

use crate::interpreter::{Env, EvalError};

use super::parser::{parse_exp, parse_variable_name};

/// A type of a literal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Integer,
    String,
    Bool,
    Float,
    Null,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer => write!(f, "int"),
            Self::String => write!(f, "string"),
            Self::Bool => write!(f, "bool"),
            Self::Float => write!(f, "float"),
            Self::Null => write!(f, "null"),
        }
    }
}

/// A literal value that can be used in expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(Arc<str>),
}

impl Literal {
    /// The type of the literal value.
    pub const fn type_name(&self) -> Type {
        match self {
            Self::Int(_) => Type::Integer,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::String(_) => Type::String,
        }
    }
}

impl<'a> From<&'a Literal> for bool {
    // Truthiness of a literal value:
    // - Integers are false if they are 0, true otherwise
    // - Floats are false if they are 0.0, true otherwise
    // - Booleans are their own truthiness
    // - Strings are false if they are empty, true otherwise
    fn from(lit: &'a Literal) -> Self {
        match lit {
            Literal::Int(i) => *i != 0,
            Literal::Float(f) => *f != 0.0,
            Literal::Bool(b) => *b,
            Literal::String(s) => !s.is_empty(),
        }
    }
}

impl From<Literal> for Type {
    fn from(lit: Literal) -> Self {
        lit.type_name()
    }
}

impl<'a> TryFrom<&'a Literal> for i64 {
    type Error = String;

    fn try_from(value: &'a Literal) -> Result<Self, Self::Error> {
        match value {
            Literal::Int(i) => Ok(*i),
            Literal::Float(f) => Ok(*f as Self),
            Literal::String(s) => s
                .parse()
                .map_err(|e| format!("Failed to parse string as integer: {e}")),
            Literal::Bool(b) => Ok(Self::from(*b)),
        }
    }
}

impl<'a> TryFrom<&'a Literal> for f64 {
    type Error = String;

    fn try_from(value: &'a Literal) -> Result<Self, Self::Error> {
        match value {
            Literal::Float(f) => Ok(*f),
            Literal::Int(i) => Ok(*i as Self),
            Literal::String(s) => s
                .parse()
                .map_err(|e| format!("Failed to parse string as float: {e}")),
            Literal::Bool(b) => Ok(Self::from(*b)),
        }
    }
}

impl<'a> From<&'a Literal> for Arc<str> {
    fn from(value: &'a Literal) -> Self {
        match value {
            Literal::String(s) => s.clone(),
            Literal::Int(i) => i.to_string().into(),
            Literal::Float(f) => f.to_string().into(),
            Literal::Bool(b) => b.to_string().into(),
        }
    }
}

impl TryFrom<serde_json::Value> for Literal {
    type Error = String;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::String(s) => Ok(Self::String(s.into())),
            serde_json::Value::Number(n) if n.is_i64() => {
                Ok(Self::Int(n.as_i64().expect("Failed to parse integer")))
            }
            serde_json::Value::Number(n) if n.is_f64() => {
                Ok(Self::Float(n.as_f64().expect("Failed to parse float")))
            }
            serde_json::Value::Number(n) => Err(format!("Unsupported number type: {n}")),
            serde_json::Value::Bool(b) => Ok(Self::Bool(b)),
            _ => Err(format!("Unsupported value type: {value:?}")),
        }
    }
}

/// A variable name, with an optional index for array access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarName {
    name: Box<str>,
    index: Option<usize>,
}

impl VarName {
    pub fn new(name: impl Into<Box<str>>, index: Option<usize>) -> Self {
        Self {
            name: name.into(),
            index,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn index(&self) -> Option<usize> {
        self.index
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VarAccessError {
    #[error("Variable access is empty")]
    EmptyAccess,
    #[error("Variable not found: {variable}")]
    VariableNotFound { variable: String },
    #[error("Type error: {message}")]
    TypeError { message: String },
    #[error("Index out of bounds: {message}")]
    IndexOutOfBounds { message: String },
    #[error("Conversion error: {message}")]
    ConversionError { message: String },
}

/// A variable access, which is a series of variable names.
///
/// Example: `foo.bar[0].baz` would be represented as a `VarAccess` with three `VarName`s:
/// - `foo` with no index
/// - `bar` with index 0
/// - `baz` with no index
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarAccess {
    names: Vec<VarName>,
}

impl VarAccess {
    /// Create a new `VarAccess` from a vector of `VarName`s.
    ///
    /// # Panics
    ///
    /// This function will panic if the `names` vector is empty, as a variable access must have at least one name.
    pub const fn new(names: Vec<VarName>) -> Self {
        assert!(
            !names.is_empty(),
            "Variable access must have at least one name"
        );

        Self { names }
    }

    pub fn names(&self) -> &[VarName] {
        &self.names
    }

    fn access_names(
        mut names: &[VarName],
        value: &serde_json::Value,
        ignore_first: bool,
    ) -> Result<Option<Literal>, VarAccessError> {
        let mut current = value;

        let var = names.last().ok_or(VarAccessError::EmptyAccess)?;

        if ignore_first {
            names = names.get(1..).ok_or(VarAccessError::EmptyAccess)?;
        }

        // Reduce "current" by accessing each variable name in the access path
        for var in names {
            if let serde_json::Value::Object(o) = current {
                current = o
                    .get(var.name())
                    .ok_or_else(|| VarAccessError::VariableNotFound {
                        variable: var.name().to_string(),
                    })?;

                if let Some(index) = var.index() {
                    let arr = current
                        .as_array()
                        .ok_or_else(|| VarAccessError::TypeError {
                            message: format!(
                                "Expected array at '{}', received {:?}",
                                var.name(),
                                current
                            ),
                        })?;

                    current = arr
                        .get(index)
                        .ok_or_else(|| VarAccessError::IndexOutOfBounds {
                            message: format!(
                                "Index out of bounds at '{}' (index: {index} length: {})",
                                var.name(),
                                arr.len()
                            ),
                        })?;
                }
            }
        }

        match current {
            serde_json::Value::Null => Ok(None),
            serde_json::Value::Object(_) => Err(VarAccessError::TypeError {
                message: format!("Cannot use object in expression '{}'", var.name()),
            }),
            serde_json::Value::Array(_) if var.index().is_none() => {
                Err(VarAccessError::TypeError {
                    message: format!("Cannot use array in expression '{}'", var.name()),
                })
            }
            serde_json::Value::Array(arr) => {
                let index = var.index().ok_or_else(|| VarAccessError::ConversionError {
                    message: format!("Expected array index for '{}'", var.name()),
                })?;

                let value = arr
                    .get(index)
                    .ok_or_else(|| VarAccessError::IndexOutOfBounds {
                        message: format!(
                            "Index out of bounds at '{}' (index: {index} length: {})",
                            var.name(),
                            arr.len()
                        ),
                    })?;

                Literal::try_from(value.clone()).map(Some).map_err(|e| {
                    VarAccessError::ConversionError {
                        message: format!("Failed to convert value at '{}': {e}", var.name()),
                    }
                })
            }
            v => Literal::try_from(v.clone()).map(Some).map_err(|e| {
                VarAccessError::ConversionError {
                    message: format!("Failed to convert value at '{}': {e}", var.name()),
                }
            }),
        }
    }

    /// Access the value denoted by this accessor from the given JSON value.
    ///
    /// # Returns
    /// - `Ok(Some(Literal))` if the value was successfully accessed and converted to a `Literal`
    /// - `Ok(None)` if the value was `null`
    ///
    /// # Errors
    /// - If there was an error accessing the value, such as a type mismatch or index out of bounds
    pub fn access(&self, value: &serde_json::Value) -> Result<Option<Literal>, VarAccessError> {
        Self::access_names(&self.names, value, false)
    }

    /// Access the value denoted by this accessor from the given JSON value.
    ///
    /// # Returns
    /// - `Ok(Some(Literal))` if the value was successfully accessed and converted to a `Literal`
    /// - `Ok(None)` if the value was `null`
    ///
    /// # Errors
    /// - If there was an error accessing the value, such as a type mismatch or index out of bounds
    pub fn access_from_bindings(
        &self,
        bindings: &HashMap<Box<str>, serde_json::Value>,
    ) -> Result<Option<Literal>, VarAccessError> {
        if self.names.is_empty() {
            return Ok(None);
        }

        let first_name = self.names[0].name();
        let value = bindings
            .get(first_name)
            .ok_or_else(|| VarAccessError::VariableNotFound {
                variable: first_name.to_string(),
            })?;

        Self::access_names(&self.names, value, true)
    }
}

impl Display for VarAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        for var in &self.names {
            result.push_str(var.name());
            if let Some(index) = var.index() {
                write!(result, "[{index}]").expect("Failed to write index");
            }
            result.push('.');
        }
        // Remove the trailing dot
        result.pop();
        write!(f, "{result}")
    }
}

impl<'a> TryFrom<&'a str> for VarAccess {
    type Error = nom::error::Error<&'a str>;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match parse_variable_name(s).finish() {
            Ok(("", var_access)) => Ok(var_access),
            Ok((remaining, _)) => Err(nom::error::Error::new(
                remaining,
                nom::error::ErrorKind::Eof,
            )),
            Err(e) => Err(e),
        }
    }
}

impl<'a> Deserialize<'a> for VarAccess {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Exp {
    Literal(Literal),
    Var(VarAccess),
    FnCall(FunctionItem),
    Neg(Box<Self>),
    Or(Box<Self>, Box<Self>),
    And(Box<Self>, Box<Self>),
    Eq(Box<Self>, Box<Self>),
    Neq(Box<Self>, Box<Self>),
    Gt(Box<Self>, Box<Self>),
    Lt(Box<Self>, Box<Self>),
    Geq(Box<Self>, Box<Self>),
    Leq(Box<Self>, Box<Self>),
}

impl Exp {
    /// Parse an expression from the input string and return an `Exp` enum
    ///
    /// ## Parameters
    ///
    /// - `input`: The input string to parse, e.g. "1 + 2 * 3"
    ///
    /// ## Returns
    /// - The parsed expression
    ///
    /// ## Errors
    ///
    /// - If the input string does not match the expected pattern, a parsing error will be returned.
    pub fn parse(input: &str) -> Result<Self, nom::error::Error<&str>> {
        let (remainder, exp) = parse_exp(input).finish()?;

        if !remainder.trim().is_empty() {
            return Err(nom::error::Error::new(
                remainder,
                nom::error::ErrorKind::Eof,
            ));
        }

        Ok(exp)
    }

    /// Evaluate the expression in the given environment and return the resulting literal value.
    ///
    /// ## Parameters
    ///
    /// - `env`: The environment to evaluate the expression in, which contains variable bindings and function definitions.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(Literal))` if the expression was successfully evaluated and resulted in a literal value.
    /// - `Ok(None)` if the expression was successfully evaluated but resulted in a null value
    ///
    /// ## Errors
    ///
    /// - If there was an error during evaluation, such as a type error or undefined variable, an `EvalError` will be returned.
    pub fn eval(&self, env: &Env) -> Result<Option<Cow<'_, Literal>>, EvalError> {
        super::interpreter::eval(self, env)
    }

    /// Create a new `Exp` representing a literal value.
    ///
    /// ## Parameters
    ///
    /// - `lit`: The literal value to create an expression for.
    ///
    /// ## Returns
    ///
    /// - An `Exp` enum representing the literal value.
    #[inline]
    pub const fn literal(lit: Literal) -> Self {
        Self::Literal(lit)
    }

    /// Create a new `Exp` representing a variable access.
    ///
    /// ## Parameters
    ///
    /// - `accessor`: The variable access to create an expression for.
    ///
    /// ## Returns
    ///
    /// - An `Exp` enum representing the variable access.
    #[inline]
    pub const fn var(accessor: VarAccess) -> Self {
        Self::Var(accessor)
    }

    /// Create a new `Exp` representing a function call.
    ///
    /// ## Parameters
    ///
    /// - `accessor`: The variable access syntax, e.g. "foo.bar[0].baz"
    ///
    /// ## Returns
    ///
    /// - An `Exp` enum representing the function call.
    ///
    /// ## Errors
    ///
    /// - If the variable access syntax is invalid
    #[inline]
    pub fn varname(accessor: &str) -> Result<Self, nom::error::Error<&str>> {
        VarAccess::try_from(accessor).map(Self::var)
    }

    /// Create a new `Exp` representing a function call.
    ///
    /// ## Parameters
    ///
    /// - `func`: The function to call, which includes the function name and its arguments.
    ///
    /// ## Returns
    ///
    /// - An `Exp` enum representing the function call.
    #[inline]
    pub const fn fn_call(func: FunctionItem) -> Self {
        Self::FnCall(func)
    }

    #[inline]
    #[expect(clippy::should_implement_trait)]
    pub fn neg(exp: Self) -> Self {
        Self::Neg(Box::new(exp))
    }

    #[inline]
    pub fn or(lhs: Self, rhs: Self) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn and(lhs: Self, rhs: Self) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn eq(lhs: Self, rhs: Self) -> Self {
        Self::Eq(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn neq(lhs: Self, rhs: Self) -> Self {
        Self::Neq(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn gt(lhs: Self, rhs: Self) -> Self {
        Self::Gt(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn lt(lhs: Self, rhs: Self) -> Self {
        Self::Lt(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn geq(lhs: Self, rhs: Self) -> Self {
        Self::Geq(Box::new(lhs), Box::new(rhs))
    }

    #[inline]
    pub fn leq(lhs: Self, rhs: Self) -> Self {
        Self::Leq(Box::new(lhs), Box::new(rhs))
    }
}

impl<'a> TryFrom<&'a str> for Exp {
    type Error = nom::error::Error<&'a str>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl<'a> Deserialize<'a> for Exp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionItem {
    name: String,
    args: Vec<Exp>,
}

impl FunctionItem {
    pub fn new(name: impl Into<String>, args: impl Into<Vec<Exp>>) -> Self {
        Self {
            name: name.into(),
            args: args.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn args(&self) -> &[Exp] {
        &self.args
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    static TEST_VALUE_1: LazyLock<serde_json::Value> = LazyLock::new(|| {
        serde_json::json!(
            {
                "foo": {
                    "bar": [
                        {"baz": 42},
                        {"baz": 43}
                    ]
                }
            }
        )
    });

    static TEST_VALUE_2: LazyLock<serde_json::Value> = LazyLock::new(|| {
        serde_json::json!(
            {
                "foo": {
                    "bar": [
                        {"baz": 42},
                        {"baz": 43}
                    ]
                },
                "arr": [1, 2, 3],
                "null_value": null,
                "string_value": "hello",
                "bool_value": true,
                "float_value": 3.145
            }
        )
    });

    #[test]
    fn test_var_access() {
        let var_access = VarAccess::try_from("foo.bar[0].baz").unwrap();
        let result = var_access.access(&TEST_VALUE_1).unwrap();
        assert_eq!(result, Some(Literal::Int(42)));
    }

    #[test]
    fn test_var_access_from_bindings() {
        let bindings = HashMap::from([
            ("test".into(), TEST_VALUE_1.clone()),
            ("other".into(), TEST_VALUE_2.clone()),
        ]);

        let var_access = VarAccess::try_from("test.foo.bar[1].baz").unwrap();
        let result = var_access.access_from_bindings(&bindings).unwrap();
        assert_eq!(result, Some(Literal::Int(43)));

        let var_access = VarAccess::try_from("other.arr[1]").unwrap();
        let result = var_access.access_from_bindings(&bindings).unwrap();
        assert_eq!(result, Some(Literal::Int(2)));
    }
}
