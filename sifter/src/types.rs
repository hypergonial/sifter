use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Write},
};

use nom::Finish;
use serde::Deserialize;

use crate::{VTable, VarAccessError, errors::EvalError, functions::DEFAULT_VTABLE};

use super::parser::{parse_exp, parse_variable_name};

/// A type of a literal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    /// An integer value (base 10, signed 64-bit integer).
    Integer,
    /// A UTF-8 string value.
    String,
    /// A boolean value (`true` or `false`).
    Bool,
    /// An IEEE-754 floating-point value (64-bit).
    Float,
    /// A null value, representing the absence of a value.
    /// This type has no associated data and is used to represent null literals and null values in JSON.
    /// It is distinct from other types and is considered falsy in boolean contexts.
    NullType,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer => write!(f, "int"),
            Self::String => write!(f, "string"),
            Self::Bool => write!(f, "bool"),
            Self::Float => write!(f, "float"),
            Self::NullType => write!(f, "null"),
        }
    }
}

/// A literal value that can be used in expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'a> {
    /// An integer value (base 10, signed 64-bit integer).
    Int(i64),
    /// An IEEE-754 floating-point value (64-bit).
    Float(f64),
    /// A boolean value (`true` or `false`).
    Bool(bool),
    /// A UTF-8 string value or reference.
    String(Cow<'a, str>),
    /// A null value, representing the absence of a value.
    /// This type has no associated data and is used to represent null literals and null values in JSON.
    /// It is distinct from other types and is considered falsy in boolean contexts.
    Null,
}

impl Literal<'_> {
    /// Convert this [`Literal`] into an owned version, where all borrowed data is cloned into owned data.
    ///
    /// This is useful for cases where you want to take ownership of a [`Literal`] that may contain
    /// borrowed data (e.g. from a JSON value) and ensure that it is fully owned and independent of any original data sources.
    pub fn into_owned(self) -> Literal<'static> {
        match self {
            Literal::String(s) => Literal::String(match s {
                Cow::Borrowed(b) => Cow::Owned(b.to_owned()),
                Cow::Owned(o) => Cow::Owned(o),
            }),
            Literal::Int(i) => Literal::Int(i),
            Literal::Float(f) => Literal::Float(f),
            Literal::Bool(b) => Literal::Bool(b),
            Literal::Null => Literal::Null,
        }
    }
}

impl Literal<'_> {
    /// The type of the literal value.
    pub const fn type_name(&self) -> Type {
        match self {
            Self::Int(_) => Type::Integer,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::String(_) => Type::String,
            Self::Null => Type::NullType,
        }
    }
}

impl Display for Literal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(fl) => write!(f, "{fl}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Null => write!(f, "null"),
        }
    }
}

impl<'a> From<&'a Literal<'a>> for bool {
    // Truthiness of a literal value:
    // - Integers are false if they are 0, true otherwise
    // - Floats are false if they are 0.0, true otherwise
    // - Booleans are their own truthiness
    // - Strings are false if they are empty, true otherwise
    fn from(lit: &'a Literal<'a>) -> Self {
        match lit {
            Literal::Int(i) => *i != 0,
            Literal::Float(f) => *f != 0.0,
            Literal::Bool(b) => *b,
            Literal::String(s) => !s.is_empty(),
            Literal::Null => false,
        }
    }
}

impl<'a> From<Literal<'a>> for Type {
    fn from(lit: Literal<'a>) -> Self {
        lit.type_name()
    }
}

impl<'a> TryFrom<&'a Literal<'a>> for i64 {
    type Error = String;

    fn try_from(value: &'a Literal<'a>) -> Result<Self, Self::Error> {
        match value {
            Literal::Int(i) => Ok(*i),
            Literal::Float(f) => Ok(*f as Self),
            Literal::String(s) => s
                .parse()
                .map_err(|e| format!("Failed to parse string as integer: {e}")),
            Literal::Bool(b) => Ok(Self::from(*b)),
            Literal::Null => Err("Cannot convert null to integer".into()),
        }
    }
}

impl<'a> TryFrom<&'a Literal<'a>> for f64 {
    type Error = String;

    fn try_from(value: &'a Literal<'a>) -> Result<Self, Self::Error> {
        match value {
            Literal::Float(f) => Ok(*f),
            Literal::Int(i) => Ok(*i as Self),
            Literal::String(s) => s
                .parse()
                .map_err(|e| format!("Failed to parse string as float: {e}")),
            Literal::Bool(b) => Ok(Self::from(*b)),
            Literal::Null => Err("Cannot convert null to float".into()),
        }
    }
}

impl<'a> From<&'a Literal<'a>> for Cow<'a, str> {
    fn from(value: &'a Literal<'a>) -> Self {
        match value {
            Literal::String(s) => s.clone(),
            Literal::Int(i) => i.to_string().into(),
            Literal::Float(f) => f.to_string().into(),
            Literal::Bool(b) => b.to_string().into(),
            Literal::Null => "null".into(),
        }
    }
}

impl<'de> Deserialize<'de> for Literal<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<serde_json::Value> for Literal<'_> {
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

impl<'a> TryFrom<&'a serde_json::Value> for Literal<'a> {
    type Error = String;

    fn try_from(value: &'a serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::String(s) => Ok(Self::String(Cow::Borrowed(s))),
            serde_json::Value::Number(n) if n.is_i64() => {
                Ok(Self::Int(n.as_i64().expect("Failed to parse integer")))
            }
            serde_json::Value::Number(n) if n.is_f64() => {
                Ok(Self::Float(n.as_f64().expect("Failed to parse float")))
            }
            serde_json::Value::Number(n) => Err(format!("Unsupported number type: {n}")),
            serde_json::Value::Bool(b) => Ok(Self::Bool(*b)),
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
    /// Create a new [`VarName`] with the given name and optional index.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the variable.
    /// - `index`: An optional index for array access,
    ///   if this variable name is used to access an array element
    ///   (e.g. `foo[0]` would have name "foo" and index 0).
    ///
    /// # Returns
    ///
    /// - A new [`VarName`] instance containing the provided name and index.
    pub fn new(name: impl Into<Box<str>>, index: Option<usize>) -> Self {
        Self {
            name: name.into(),
            index,
        }
    }

    /// The name of the variable.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The optional index for array access, if this variable name is used to access an array element.
    pub const fn index(&self) -> Option<usize> {
        self.index
    }
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
    /// Create a new [`VarAccess`] from a vector of [`VarName`]s.
    ///
    /// # Panics
    ///
    /// This function will panic if the `names` vector is empty, as a variable access must have at least one name.
    pub fn new(names: impl Into<Vec<VarName>>) -> Self {
        let names = names.into();
        assert!(
            !names.is_empty(),
            "Variable access must have at least one name"
        );

        Self { names }
    }

    /// Get the sequence variable names in this access.
    pub fn names(&self) -> &[VarName] {
        &self.names
    }

    fn access_names<'a>(
        mut names: &[VarName],
        value: &'a serde_json::Value,
        ignore_first: bool,
    ) -> Result<Option<Literal<'a>>, VarAccessError> {
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

                Literal::try_from(value)
                    .map(Some)
                    .map_err(|e| VarAccessError::ConversionError {
                        message: format!("Failed to convert value at '{}': {e}", var.name()),
                    })
            }
            v => Literal::try_from(v)
                .map(Some)
                .map_err(|e| VarAccessError::ConversionError {
                    message: format!("Failed to convert value at '{}': {e}", var.name()),
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
    pub fn access<'a>(
        &self,
        value: &'a serde_json::Value,
    ) -> Result<Option<Literal<'a>>, VarAccessError> {
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
    pub fn access_from_bindings<'a>(
        &self,
        env: &'a Env<'a>,
    ) -> Result<Option<Literal<'a>>, VarAccessError> {
        if self.names.is_empty() {
            return Ok(None);
        }

        let first_name = self.names[0].name();
        let value =
            env.bindings()
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

/// Represents an Abstract Syntax Tree (AST) for sifter expressions,
/// which can be evaluated in a given environment to produce a literal value.
#[derive(Debug, Clone, PartialEq)]
pub enum Exp<'a> {
    Literal(Literal<'a>),
    FnCall(FunctionItem<'a>),
    Var(VarAccess),
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

impl<'a> Exp<'a> {
    /// Create a new [`Exp`] from a string representation of an expression.
    ///
    /// # Parameters
    ///
    /// - `string`: The string representation of the expression to parse.
    ///
    /// # Returns
    ///
    /// - <code>Ok([`Exp`])</code> if the expression was successfully parsed from the string.
    ///
    /// # Errors
    ///
    /// - If there was an error parsing the expression from the string,
    ///   such as a syntax error, an `Err` will be returned containing the parsing error details.
    ///
    ///   Note that semantic errors (e.g. undefined variables, type errors) are not handled by this
    ///   function and will not result in an error being returned here. Those errors will be encountered
    ///   during evaluation of the expression, and will be returned as [`EvalError`]s from the [`Exp::eval`] method.
    pub fn new(string: impl Into<&'a str>) -> Result<Self, nom::error::Error<String>> {
        string.into().try_into()
    }

    /// Turn the expression into an owned version, where all borrowed data is cloned into owned data.
    ///
    /// This is useful for cases where you want to take ownership of an [`Exp`] that may contain
    /// borrowed data (e.g. from a JSON value) and ensure that it is fully owned and independent of any original data sources.
    ///
    /// Note that this will recursively clone all borrowed data in the expression, so it may be expensive for large expressions with a lot of borrowed data.
    /// However, if the expression is already fully owned, this will simply return a clone of the expression without any additional cloning of data.
    ///
    /// # Returns
    ///
    /// - An owned version of this expression, where all borrowed data has been cloned into owned data.
    pub fn into_owned(self) -> Exp<'static> {
        match self {
            Exp::Literal(lit) => Exp::Literal(lit.into_owned()),
            Exp::FnCall(func) => Exp::FnCall(FunctionItem {
                name: func.name,
                args: func.args.into_iter().map(Exp::into_owned).collect(),
            }),
            Exp::Var(var) => Exp::Var(var),
            Exp::Neg(e) => Exp::Neg(Box::new(e.into_owned())),
            Exp::Or(l, r) => Exp::Or(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::And(l, r) => Exp::And(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Eq(l, r) => Exp::Eq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Neq(l, r) => Exp::Neq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Gt(l, r) => Exp::Gt(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Lt(l, r) => Exp::Lt(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Geq(l, r) => Exp::Geq(Box::new(l.into_owned()), Box::new(r.into_owned())),
            Exp::Leq(l, r) => Exp::Leq(Box::new(l.into_owned()), Box::new(r.into_owned())),
        }
    }

    /// Evaluate the expression in the given environment and return the resulting literal value.
    ///
    /// ## Parameters
    ///
    /// - `env`: The [`Env`] to evaluate the expression in, which contains variable bindings and function definitions.
    ///
    /// ## Returns
    ///
    /// - <code>Ok([Cow]<'_, [Literal]>)</code> if the expression was successfully evaluated, where the `Literal` is the resulting value of the expression.
    ///
    /// ## Errors
    ///
    /// - If there was an error during evaluation, such as a type error or undefined variable, an [`EvalError`] will be returned.
    pub fn eval<'b, 'c>(&'a self, env: &'b Env<'b>) -> Result<Cow<'c, Literal<'c>>, EvalError>
    where
        'a: 'c,
        'b: 'c,
    {
        super::interpreter::eval(self, env)
    }

    /// Create a new [`Exp`] representing a literal value.
    ///
    /// ## Parameters
    ///
    /// - `lit`: The literal value to create an expression for.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the literal value.
    #[inline]
    pub const fn literal(lit: Literal<'a>) -> Self {
        Self::Literal(lit)
    }

    /// Create a new [`Exp`] representing a variable access.
    ///
    /// ## Parameters
    ///
    /// - `accessor`: The variable access to create an expression for.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the variable access.
    #[inline]
    pub const fn var(accessor: VarAccess) -> Self {
        Self::Var(accessor)
    }

    /// Create a new [`Exp`] representing a function call.
    ///
    /// ## Parameters
    ///
    /// - `accessor`: The variable access syntax, e.g. "foo.bar[0].baz"
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the function call.
    ///
    /// ## Errors
    ///
    /// - If the variable access syntax is invalid
    #[inline]
    pub fn varname(accessor: &str) -> Result<Self, nom::error::Error<&str>> {
        VarAccess::try_from(accessor).map(Self::var)
    }

    /// Create a new [`Exp`] representing a function call.
    ///
    /// ## Parameters
    ///
    /// - `func`: The function to call, which includes the function name and its arguments.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the function call.
    #[inline]
    pub const fn fn_call(func: FunctionItem<'a>) -> Self {
        Self::FnCall(func)
    }

    /// Create a new [`Exp`] representing a negation of another expression.
    ///
    /// ## Parameters
    ///
    /// - `exp`: The expression to negate.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the negation of the given expression.
    #[inline]
    #[expect(clippy::should_implement_trait)]
    pub fn neg(exp: Self) -> Self {
        Self::Neg(Box::new(exp))
    }

    /// Create a new [`Exp`] representing a logical OR of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the OR operation.
    /// - `rhs`: The right-hand side expression of the OR operation.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the logical OR of the two given expressions.
    #[inline]
    pub fn or(lhs: Self, rhs: Self) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a logical AND of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the AND operation.
    /// - `rhs`: The right-hand side expression of the AND operation.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the logical AND of the two given expressions.
    #[inline]
    pub fn and(lhs: Self, rhs: Self) -> Self {
        Self::And(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing an equality comparison of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the equality comparison.
    /// - `rhs`: The right-hand side expression of the equality comparison.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the equality comparison of the two given expressions.
    #[inline]
    pub fn eq(lhs: Self, rhs: Self) -> Self {
        Self::Eq(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing an inequality comparison of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the inequality comparison.
    /// - `rhs`: The right-hand side expression of the inequality comparison.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the inequality comparison of the two given expressions.
    #[inline]
    pub fn neq(lhs: Self, rhs: Self) -> Self {
        Self::Neq(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a greater-than comparison of two expressions.
    ///
    /// ## Parameters
    ///
    /// - `lhs`: The left-hand side expression of the greater-than comparison.
    /// - `rhs`: The right-hand side expression of the greater-than comparison.
    ///
    /// ## Returns
    ///
    /// - An [`Exp`] enum representing the greater-than comparison of the two given expressions.
    #[inline]
    pub fn gt(lhs: Self, rhs: Self) -> Self {
        Self::Gt(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a less-than comparison of two expressions.
    ///
    /// ## Parameters
    /// - `lhs`: The left-hand side expression of the less-than comparison.
    /// - `rhs`: The right-hand side expression of the less-than comparison.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the less-than comparison of the two given expressions.
    #[inline]
    pub fn lt(lhs: Self, rhs: Self) -> Self {
        Self::Lt(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a greater-than-or-equal-to comparison of two expressions.
    ///
    /// ## Parameters
    /// - `lhs`: The left-hand side expression of the greater-than-or-equal-to comparison.
    /// - `rhs`: The right-hand side expression of the greater-than-or-equal-to comparison.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the greater-than-or-equal-to comparison of the two given expressions.
    #[inline]
    pub fn geq(lhs: Self, rhs: Self) -> Self {
        Self::Geq(Box::new(lhs), Box::new(rhs))
    }

    /// Create a new [`Exp`] representing a less-than-or-equal-to comparison of two expressions.
    ///
    /// ## Parameters
    /// - `lhs`: The left-hand side expression of the less-than-or-equal-to comparison.
    /// - `rhs`: The right-hand side expression of the less-than-or-equal-to comparison.
    ///
    /// ## Returns
    /// - An [`Exp`] enum representing the less-than-or-equal-to comparison of the two given expressions.
    #[inline]
    pub fn leq(lhs: Self, rhs: Self) -> Self {
        Self::Leq(Box::new(lhs), Box::new(rhs))
    }
}

impl<'a> TryFrom<&'a str> for Exp<'a> {
    type Error = nom::error::Error<String>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (remainder, exp) = parse_exp(value).finish()?;

        if !remainder.trim().is_empty() {
            return Err(nom::error::Error::new(
                remainder.to_string(),
                nom::error::ErrorKind::Eof,
            ));
        }

        Ok(exp)
    }
}

impl<'a> Deserialize<'a> for Exp<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

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

/// The evaluation environment for a sifter expression, containing variable bindings and a function vtable.
///
/// To construct an `Env`, use `Env::new()` to create an [`EnvBuilder`], which provides a fluent interface
/// for adding variable bindings and configuring the vtable. Once all bindings and configuration are set,
/// call `.build()` on the [`EnvBuilder`] to create the final [`Env`] instance.
#[derive(Debug, Clone)]
pub struct Env<'var> {
    bindings: HashMap<Box<str>, Cow<'var, serde_json::Value>>,
    vtable: VTable,
}

impl<'var> Env<'var> {
    /// Create a new [`EnvBuilder`] for constructing an [`Env`].
    ///
    /// # Example
    /// ```rust
    /// use sifter::Env;
    /// let env = Env::new()
    ///     .bind("x", serde_json::json!(42))
    ///     .bind("y", serde_json::json!("hello"))
    ///     .build();
    ///
    /// assert_eq!(env.bindings().get("x").unwrap().as_ref(), &serde_json::json!(42));
    /// assert_eq!(env.bindings().get("y").unwrap().as_ref(), &serde_json::json!("hello"));
    /// ```
    #[expect(clippy::new_ret_no_self)]
    pub fn new() -> EnvBuilder<'var> {
        EnvBuilder::new()
    }

    /// Get a reference to the variable bindings in this environment.
    ///
    /// # Returns
    ///
    /// - A reference to the variable bindings, which is a `HashMap` mapping variable names
    ///   to their corresponding JSON values.
    #[inline]
    pub const fn bindings(&self) -> &HashMap<Box<str>, Cow<'var, serde_json::Value>> {
        &self.bindings
    }

    /// Get a reference to the active vtable.
    ///
    /// # Returns
    ///
    /// - A reference to the `VTable` containing the function definitions available in this environment.
    #[inline]
    pub(super) const fn vtable(&self) -> &VTable {
        &self.vtable
    }
}

/// A builder to construct an [`Env`].
///
/// # Example
/// ```rust
/// use sifter::Env;
/// let env = Env::new()
///     .bind("x", serde_json::json!(42))
///     .bind("y", serde_json::json!("hello"))
///     .build();
///
/// assert_eq!(env.bindings().get("x").unwrap().as_ref(), &serde_json::json!(42));
/// assert_eq!(env.bindings().get("y").unwrap().as_ref(), &serde_json::json!("hello"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct EnvBuilder<'var> {
    bindings: HashMap<Box<str>, Cow<'var, serde_json::Value>>,
    vtable: Option<VTable>,
}

impl<'var> EnvBuilder<'var> {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            vtable: None,
        }
    }

    /// Returns true if the given variable name is bound in this environment, false otherwise.
    pub fn is_bound(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Get a reference to the value bound to the given variable name, if it exists.
    ///
    /// # Parameters
    /// - `name`: The name of the variable to look up.
    ///
    /// # Returns
    ///
    /// - `Some(&Cow<'var, serde_json::Value>)` if the variable is bound in this environment,
    ///   where the `Cow` contains a reference to the value if it was bound using `bind_ref`,
    ///   or an owned value if it was bound using `bind`.
    /// - `None` if the variable is not bound in this environment.
    pub fn get_binding(&self, name: &str) -> Option<&Cow<'var, serde_json::Value>> {
        self.bindings.get(name)
    }

    /// Bind a variable name to a JSON value in this environment.
    /// If you want to bind a reference instead of an owned value, see [`EnvBuilder::bind_ref`].
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the variable to bind.
    /// - `value`: The JSON value to bind to the variable name.
    ///
    /// # Returns
    ///
    /// - A mutable reference to this [`EnvBuilder`] for method chaining.
    pub fn bind(&mut self, name: impl Into<Box<str>>, value: serde_json::Value) -> &mut Self {
        self.bindings.insert(name.into(), Cow::Owned(value));
        self
    }

    /// Bind multiple variable names to JSON values in this environment.
    /// If you want to bind references instead of owned values, see [`EnvBuilder::bind_ref_multiple`].
    ///
    /// # Parameters
    ///
    /// - `vars`: An iterable of `(name, value)` pairs, where `name` is the variable name to bind
    ///   and `value` is the JSON value to bind to that name.
    ///
    /// # Returns
    ///
    /// - A mutable reference to this [`EnvBuilder`] for method chaining.
    pub fn bind_multiple(
        &mut self,
        vars: impl IntoIterator<Item = (impl Into<Box<str>>, serde_json::Value)>,
    ) -> &mut Self {
        for (name, value) in vars {
            self.bindings.insert(name.into(), Cow::Owned(value));
        }
        self
    }

    /// Bind a reference to a JSON value in this environment, which allows the value
    /// to be shared across multiple environments without cloning. Additionally, when possible,
    /// the return value of an evaluation can be a reference to one of the bindings or literals,
    /// which can be more efficient than returning an owned value.
    /// If you want to bind an owned value instead of a reference, see [`EnvBuilder::bind`].
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the variable to bind.
    /// - `value`: A reference to the JSON value to bind to the variable name.
    ///
    /// # Returns
    ///
    /// - A mutable reference to this [`EnvBuilder`] for method chaining.
    pub fn bind_ref(
        &mut self,
        name: impl Into<Box<str>>,
        value: &'var serde_json::Value,
    ) -> &mut Self {
        self.bindings.insert(name.into(), Cow::Borrowed(value));
        self
    }

    /// Bind multiple references to JSON values in this environment, which allows the values
    /// to be shared across multiple environments without cloning. Additionally, when possible,
    /// the return value of an evaluation can be a reference to one of the bindings or literals,
    /// which can be more efficient than returning an owned value.
    /// If you want to bind owned values instead of references, see [`EnvBuilder::bind_multiple`].
    ///
    /// # Parameters
    ///
    /// - `vars`: An iterable of `(name, value)` pairs, where `name` is the variable name to bind
    ///   and `value` is a reference to the JSON value to bind to that name.
    ///
    /// # Returns
    ///
    /// - A mutable reference to this [`EnvBuilder`] for method chaining.
    pub fn bind_ref_multiple(
        &mut self,
        vars: impl IntoIterator<Item = (impl Into<Box<str>>, &'var serde_json::Value)>,
    ) -> &mut Self {
        for (name, value) in vars {
            self.bindings.insert(name.into(), Cow::Borrowed(value));
        }
        self
    }

    /// Use a custom vtable for this environment instead of the default one.
    /// This allows you to override the default function definitions or add new ones.
    ///
    /// Tip: You can create a custom vtable by cloning the default one and modifying it, e.g.:
    /// ```rust
    /// use sifter::{Literal, Env, VTable, DEFAULT_VTABLE, FnArgs, FnResult, FnCallError, EvalError};
    ///
    /// fn my_func(args: FnArgs<'_>) -> FnResult<'_> {
    ///     // Your function implementation goes here
    ///     if args.is_empty() {
    ///         return Err(FnCallError {
    ///             fn_name: "my_func".to_string(),
    ///             reason: EvalError::ArgumentCount {
    ///                 expected: 0,
    ///                 got: args.len(),
    ///             }
    ///             .into(),
    ///         });
    ///     }
    ///
    ///     Ok(Literal::Int(42))
    /// }
    ///
    /// let mut custom_vtable = DEFAULT_VTABLE.clone();
    /// custom_vtable.insert("my_func", my_func);
    /// let env = Env::new()
    ///    .use_vtable(custom_vtable)
    ///   .build();
    /// ```
    ///
    /// # Parameters
    ///
    /// - `vtable`: The custom vtable to use for this environment.
    ///
    /// # Returns
    ///
    /// - A mutable reference to this [`EnvBuilder`] for method chaining.
    pub fn use_vtable(&mut self, vtable: VTable) -> &mut Self {
        self.vtable = Some(vtable);
        self
    }

    /// Finish the construction of the [`Env`] and return the final instance.
    ///
    /// This will clone the variable bindings and vtable from this builder into the new `Env`.
    /// However, since `EnvBuilder` is typically dropped after this, Rust is likely to optimize
    /// away the cloning of the bindings and vtable in release mode, so this should not have a
    /// significant performance impact in practice.
    ///
    /// # Returns
    ///
    /// - An [`Env`] instance containing the variable bindings and vtable configured in this builder.
    #[must_use]
    pub fn build(&mut self) -> Env<'var> {
        // Rust is likely to optimize away the .clone() here since EnvBuilder is typically dropped after this
        // See: https://docs.rs/derive_builder/0.20.2/derive_builder/#-performance-considerations
        let vtable = self
            .vtable
            .clone()
            .unwrap_or_else(|| DEFAULT_VTABLE.clone());

        Env {
            bindings: self.bindings.clone(),
            vtable,
        }
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
        let env = Env::new()
            .bind_ref("test", &TEST_VALUE_1)
            .bind_ref("other", &TEST_VALUE_2)
            .build();

        let var_access = VarAccess::try_from("test.foo.bar[1].baz").unwrap();
        let result = var_access.access_from_bindings(&env).unwrap();
        assert_eq!(result, Some(Literal::Int(43)));

        let var_access = VarAccess::try_from("other.arr[1]").unwrap();
        let result = var_access.access_from_bindings(&env).unwrap();
        assert_eq!(result, Some(Literal::Int(2)));
    }
}
