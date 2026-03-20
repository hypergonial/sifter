use std::fmt::{Display, Write};

use nom::Finish;
use serde::Deserialize;

use crate::{
    VarAccessError,
    types::{env::Env, literal::Literal},
};

use crate::parser::parse_variable_name;

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
        let env = Env::<serde_json::Value>::new()
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
