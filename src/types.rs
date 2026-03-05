use std::{
    collections::HashMap,
    fmt::{Display, Write},
    sync::Arc,
};

use nom::IResult;

use super::parser::{parse_exp, parse_variable_name};

/// A type of a literal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Integer,
    String,
    Bool,
    Float,
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

impl From<Literal> for Type {
    fn from(lit: Literal) -> Self {
        lit.type_name()
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// A variable access, which is a series of variable names.
///
/// Example: `foo.bar[0].baz` would be represented as a `VarAccess` with three `VarName`s:
/// - `foo` with no index
/// - `bar` with index 0
/// - `baz` with no index
#[derive(Debug, Clone, PartialEq, Eq)]
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
        names: &[VarName],
        value: &serde_json::Value,
    ) -> Result<Option<Literal>, String> {
        let mut current = value;

        // Reduce "current" by accessing each variable name in the access path
        for var in names {
            if let serde_json::Value::Object(o) = current {
                current = o
                    .get(var.name())
                    .ok_or_else(|| format!("Expected object at '{}'", var.name()))?;

                if let Some(index) = var.index() {
                    let arr = current.as_array().ok_or_else(|| {
                        format!("Expected array at '{}', received {:?}", var.name(), current)
                    })?;

                    current = arr.get(index).ok_or_else(|| {
                        format!(
                            "Index out of bounds at '{}' (index: {index} length: {})",
                            var.name(),
                            arr.len()
                        )
                    })?;
                }
            }
        }

        let var = names
            .last()
            .expect("Variable access must have at least one name");

        match current {
            serde_json::Value::Null => Ok(None),
            serde_json::Value::Object(_) => {
                Err(format!("Cannot use object in expression '{}'", var.name()))
            }
            serde_json::Value::Array(_) if var.index().is_none() => {
                Err(format!("Cannot use array in expression '{}'", var.name()))
            }
            serde_json::Value::Array(arr) => {
                let index = var
                    .index()
                    .ok_or_else(|| format!("Expected array index for '{}'", var.name()))?;

                let value = arr.get(index).ok_or_else(|| {
                    format!(
                        "Index out of bounds at '{}' (index: {index} length: {})",
                        var.name(),
                        arr.len()
                    )
                })?;

                Literal::try_from(value.clone())
                    .map(Some)
                    .map_err(|e| format!("Failed to convert value at '{}': {e}", var.name()))
            }
            v => Literal::try_from(v.clone())
                .map(Some)
                .map_err(|e| format!("Failed to convert value at '{}': {e}", var.name())),
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
    pub fn access(&self, value: &serde_json::Value) -> Result<Option<Literal>, String> {
        Self::access_names(&self.names, value)
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
    ) -> Result<Option<Literal>, String> {
        if self.names.is_empty() {
            return Ok(None);
        }

        let first_name = self.names[0].name();
        let value = bindings
            .get(first_name)
            .ok_or_else(|| format!("Variable '{first_name}' not found in bindings"))?;

        Self::access_names(&self.names[1..], value)
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
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match parse_variable_name(s) {
            Ok((_, var_access)) => Ok(var_access),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Exp {
    Literal(Literal),
    Var(VarAccess),
    FnCall(Function),
    Not(Box<Self>),
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
    pub fn parse(input: &str) -> IResult<&str, Self> {
        parse_exp(input)
    }

    #[inline]
    pub fn literal(lit: Literal) -> Self {
        Self::Literal(lit)
    }

    #[inline]
    pub const fn var(accessor: VarAccess) -> Self {
        Self::Var(accessor)
    }

    #[inline]
    pub fn varname(name: &str) -> Result<Self, nom::Err<nom::error::Error<&str>>> {
        VarAccess::try_from(name).map(Self::var)
    }

    #[inline]
    pub const fn fn_call(func: Function) -> Self {
        Self::FnCall(func)
    }

    #[inline]
    #[expect(clippy::should_implement_trait)]
    pub fn neg(exp: Self) -> Self {
        Self::Not(Box::new(exp))
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
    type Error = nom::Err<nom::error::Error<&'a str>>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match Self::parse(value) {
            Ok((_, exp)) => Ok(exp),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    name: String,
    inputs: Vec<Exp>,
}

impl Function {
    pub fn new(name: impl Into<String>, inputs: Vec<Exp>) -> Self {
        Self {
            name: name.into(),
            inputs,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn inputs(&self) -> &[Exp] {
        &self.inputs
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
