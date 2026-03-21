use std::{borrow::Cow, fmt::Display};

use serde::Deserialize;

use crate::types::jsonobj::JsonObject;

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

    /// Create a new [`Literal`] from a JSON value, trying to convert it to the most specific literal type possible.
    ///
    /// # Errors
    ///
    /// If the JSON value is an object or array, since these cannot be represented as literals.
    pub fn from_json_object<V: JsonObject>(value: &V) -> Result<Literal<'_>, String> {
        if let Some(s) = value.as_str() {
            Ok(Literal::String(Cow::Borrowed(s)))
        } else if let Some(i) = value.as_i64() {
            Ok(Literal::Int(i))
        } else if let Some(f) = value.as_f64() {
            Ok(Literal::Float(f))
        } else if let Some(b) = value.as_bool() {
            Ok(Literal::Bool(b))
        } else if value.as_null().is_some() {
            Ok(Literal::Null)
        } else {
            Err("Unsupported value type".into())
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
