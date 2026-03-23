use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::{Debug, Display},
};

#[cfg(feature = "serde")]
use serde::Deserialize;

use crate::{JsonMap, JsonValue, utils::escape_str_for_json};

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
    /// An array of values
    Array,
    /// A mapping of string keys to values
    Object,
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
            Self::Array => write!(f, "array"),
            Self::Object => write!(f, "object"),
        }
    }
}

/// A literal value that can be used in expressions.
///
/// This type represents the various literal values that can be used in expressions,
/// such as integers, strings, booleans, floats, arrays, objects, and null.
///
/// It is designed to be flexible and can contain borrowed references
/// to data (e.g. from JSON values) or owned data.
///
/// The lifetime parameter `'a` is used for borrowing data from either bindings or the expression itself,
/// which allows for efficient handling of JSON values without unnecessary cloning.
///
/// To obtain a fully owned version of a [`Value`], you can use the [`Value::into_owned`] method,
/// which will clone any borrowed data and return a new [`Value`] with a `'static` lifetime.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// An integer value (base 10, signed 64-bit integer).
    Int(i64),
    /// An IEEE-754 floating-point value (64-bit).
    Float(f64),
    /// A boolean value (`true` or `false`).
    Bool(bool),
    /// A UTF-8 string value or reference.
    String(Cow<'a, str>),
    /// A sequence of values
    Array(Vec<Self>),
    /// A mapping of string keys to values
    Object(BTreeMap<String, Self>),
    /// A null value, representing the absence of a value.
    /// This type has no associated data and is used to represent null literals and null values in JSON.
    /// It is distinct from other types and is considered falsy in boolean contexts.
    Null,
}

impl Value<'_> {
    /// Convert this [`Value`] into an owned version,
    /// all borrowed data will be cloned and the resulting [`Value`] will have a `'static` lifetime.
    ///
    /// This is useful for cases where you want to take ownership of a [`Value`] that may contain
    /// borrowed data (e.g. from a JSON value) and ensure that it is fully owned and independent of any original data sources.
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::String(s) => Value::String(match s {
                Cow::Borrowed(b) => Cow::Owned(b.to_owned()),
                Cow::Owned(o) => Cow::Owned(o),
            }),
            Value::Int(i) => Value::Int(i),
            Value::Float(f) => Value::Float(f),
            Value::Bool(b) => Value::Bool(b),
            Value::Array(v) => Value::Array(v.into_iter().map(Value::into_owned).collect()),
            Value::Object(m) => {
                Value::Object(m.into_iter().map(|(k, v)| (k, v.into_owned())).collect())
            }
            Value::Null => Value::Null,
        }
    }

    /// Create a new [`Value`] from a JSON value reference,
    /// trying to convert it to the most specific literal type possible.
    ///
    /// # Returns
    ///
    /// The resulting [`Value`] value, which may contain borrowed references to the original JSON value's data.
    ///
    /// # Errors
    ///
    /// If the JSON value is an unsupported type (e.g. a JSON number that is not an integer or float),
    /// this function will return an error.
    pub fn from_json_object_ref<V: JsonValue>(value: &V) -> Result<Value<'_>, String> {
        if let Some(s) = value.as_str() {
            Ok(Value::String(Cow::Borrowed(s)))
        } else if let Some(i) = value.as_i64() {
            Ok(Value::Int(i))
        } else if let Some(f) = value.as_f64() {
            Ok(Value::Float(f))
        } else if let Some(b) = value.as_bool() {
            Ok(Value::Bool(b))
        } else if value.as_null().is_some() {
            Ok(Value::Null)
        } else if let Some(v) = value.as_array() {
            Ok(Value::Array(
                v.iter()
                    .map(Self::from_json_object_ref)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        } else if let Some(m) = value.as_object() {
            let new = m
                .iter()
                .map(|(k, v)| (k.clone(), Self::from_json_object_ref(v)))
                .map(|(k, rv)| rv.map(|v| (k, v)))
                .collect::<Result<BTreeMap<_, _>, _>>()?;

            Ok(Value::Object(new))
        } else {
            Err("Unsupported value type".into())
        }
    }

    /// Create a new [`Value`] from a JSON value, trying to convert it to the most specific literal type possible.
    ///
    /// # Returns
    ///
    /// The resulting [`Value`] value.
    ///
    /// # Errors
    ///
    /// If the JSON value is an unsupported type (e.g. a JSON number that is not an integer or float),
    /// this function will return an error.
    #[expect(clippy::missing_panics_doc)]
    pub fn from_json_object<'a, V: JsonValue + 'a>(value: V) -> Result<Value<'a>, String> {
        if value.is_string() {
            Ok(Value::String(Cow::Owned(
                value.into_string().expect("Expected JSON string"),
            )))
        } else if let Some(i) = value.as_i64() {
            Ok(Value::Int(i))
        } else if let Some(f) = value.as_f64() {
            Ok(Value::Float(f))
        } else if let Some(b) = value.as_bool() {
            Ok(Value::Bool(b))
        } else if value.as_null().is_some() {
            Ok(Value::Null)
        } else if value.is_array() {
            Ok(Value::Array(
                value
                    .into_array()
                    .expect("Expected JSON array")
                    .into_iter()
                    .map(|v| Self::from_json_object(v))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        } else if value.is_object() {
            let new = value
                .into_object()
                .expect("Expected JSON object")
                .into_iter()
                .map(|(k, v)| (k, Self::from_json_object(v)))
                .map(|(k, rv)| rv.map(|v| (k, v)))
                .collect::<Result<BTreeMap<_, _>, _>>()?;

            Ok(Value::Object(new))
        } else {
            Err("Unsupported value type".into())
        }
    }

    pub(crate) fn from_json_object_cow<V: JsonValue + Clone>(
        value: Cow<'_, V>,
    ) -> Result<Value<'_>, String> {
        match value {
            Cow::Borrowed(b) => Self::from_json_object_ref(b),
            Cow::Owned(o) => Self::from_json_object(o),
        }
    }
}

impl Value<'_> {
    /// The type of the literal value.
    pub const fn type_name(&self) -> Type {
        match self {
            Self::Int(_) => Type::Integer,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::String(_) => Type::String,
            Self::Null => Type::NullType,
            Self::Array(_) => Type::Array,
            Self::Object(_) => Type::Object,
        }
    }
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(fl) => write!(f, "{fl}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::String(s) => write!(f, "{s}"),
            Self::Null => write!(f, "null"),
            Self::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Self::String(s) = item {
                        write!(f, "\"{}\"", escape_str_for_json(s))?;
                    } else {
                        write!(f, "{item}")?;
                    }
                }
                write!(f, "]")
            }
            Self::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Self::String(s) = v {
                        write!(f, "\"{k}\": \"{}\"", escape_str_for_json(s))?;
                    } else {
                        write!(f, "\"{k}\": {v}")?;
                    }
                }
                write!(f, "}}")
            }
        }
    }
}

impl<'a> From<&'a Value<'a>> for bool {
    // Truthiness of a literal value:
    // - Integers are false if they are 0, true otherwise
    // - Floats are false if they are 0.0, true otherwise
    // - Booleans are their own truthiness
    // - Strings are false if they are empty, true otherwise
    fn from(lit: &'a Value<'a>) -> Self {
        match lit {
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
        }
    }
}

impl<'a> From<Value<'a>> for Type {
    fn from(lit: Value<'a>) -> Self {
        lit.type_name()
    }
}

impl<'a> TryFrom<&'a Value<'a>> for i64 {
    type Error = String;

    fn try_from(value: &'a Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Int(i) => Ok(*i),
            Value::Float(f) => Ok(*f as Self),
            Value::String(s) => s
                .parse()
                .map_err(|e| format!("Failed to parse string as integer: {e}")),
            Value::Bool(b) => Ok(Self::from(*b)),
            Value::Null => Err("Cannot convert null to integer".into()),
            Value::Array(_) => Err("Cannot convert array to integer".into()),
            Value::Object(_) => Err("Cannot convert object to integer".into()),
        }
    }
}

impl<'a> TryFrom<&'a Value<'a>> for f64 {
    type Error = String;

    fn try_from(value: &'a Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as Self),
            Value::String(s) => s
                .parse()
                .map_err(|e| format!("Failed to parse string as float: {e}")),
            Value::Bool(b) => Ok(Self::from(*b)),
            Value::Null => Err("Cannot convert null to float".into()),
            Value::Array(_) => Err("Cannot convert array to float".into()),
            Value::Object(_) => Err("Cannot convert object to float".into()),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Value<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl TryFrom<serde_json::Value> for Value<'_> {
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
            serde_json::Value::Null => Ok(Self::Null),
            serde_json::Value::Array(arr) => Ok(Self::Array(
                arr.into_iter()
                    .map(Self::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            serde_json::Value::Object(obj) => {
                let new = obj
                    .into_iter()
                    .map(|(k, v)| (k, Self::try_from(v)))
                    .map(|(k, rv)| rv.map(|v| (k, v)))
                    .collect::<Result<BTreeMap<_, _>, _>>()?;

                Ok(Self::Object(new))
            }
        }
    }
}
