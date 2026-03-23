use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
};

use crate::Literal;

/// A trait representing a JSON object map, which is a mapping from string keys to JSON values.
pub trait JsonMap<V: JsonValue> {
    fn get(&self, key: &str) -> Option<&V>;

    fn get_mut(&mut self, key: &str) -> Option<&mut V>;

    fn insert(&mut self, key: String, value: V) -> Option<V>;

    fn contains_key(&self, key: &str) -> bool;

    fn get_key_value(&self, key: &str) -> Option<(&String, &V)>;

    fn remove(&mut self, key: &str) -> Option<V>;

    fn remove_entry(&mut self, key: &str) -> Option<(String, V)>;
}

#[cfg(feature = "serde")]
impl JsonMap<serde_json::Value> for serde_json::Map<String, serde_json::Value> {
    fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut serde_json::Value> {
        self.get_mut(key)
    }

    fn insert(&mut self, key: String, value: serde_json::Value) -> Option<serde_json::Value> {
        self.insert(key, value)
    }

    fn contains_key(&self, key: &str) -> bool {
        self.contains_key(key)
    }

    fn get_key_value(&self, key: &str) -> Option<(&String, &serde_json::Value)> {
        self.get_key_value(key)
    }

    fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.remove(key)
    }

    fn remove_entry(&mut self, key: &str) -> Option<(String, serde_json::Value)> {
        self.remove_entry(key)
    }
}

impl<V: JsonValue, S: BuildHasher> JsonMap<V> for HashMap<String, V, S> {
    fn get(&self, key: &str) -> Option<&V> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        self.get_mut(key)
    }

    fn insert(&mut self, key: String, value: V) -> Option<V> {
        self.insert(key, value)
    }

    fn contains_key(&self, key: &str) -> bool {
        self.contains_key(key)
    }

    fn get_key_value(&self, key: &str) -> Option<(&String, &V)> {
        self.get_key_value(key)
    }

    fn remove(&mut self, key: &str) -> Option<V> {
        self.remove(key)
    }

    fn remove_entry(&mut self, key: &str) -> Option<(String, V)> {
        self.remove_entry(key)
    }
}

impl<V: JsonValue> JsonMap<V> for BTreeMap<String, V> {
    fn get(&self, key: &str) -> Option<&V> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        self.get_mut(key)
    }

    fn insert(&mut self, key: String, value: V) -> Option<V> {
        self.insert(key, value)
    }

    fn contains_key(&self, key: &str) -> bool {
        self.contains_key(key)
    }

    fn get_key_value(&self, key: &str) -> Option<(&String, &V)> {
        self.get_key_value(key)
    }

    fn remove(&mut self, key: &str) -> Option<V> {
        self.remove(key)
    }

    fn remove_entry(&mut self, key: &str) -> Option<(String, V)> {
        self.remove_entry(key)
    }
}

/// A trait representing a JSON value, which can be one of several types (object, array, string, number, boolean, or null).
pub trait JsonValue: Sized {
    /// The type of JSON object map used by this JSON value type.
    type MapType: JsonMap<Self>;

    /// Try to interpret this JSON value as an object, returning a reference to the underlying map if successful.
    fn as_object(&self) -> Option<&Self::MapType>;

    /// Try to interpret this JSON value as an object, returning a mutable reference to the underlying map if successful.
    fn as_object_mut(&mut self) -> Option<&mut Self::MapType>;

    /// Try to interpret this JSON value as an array, returning a reference to the underlying vector if successful.
    fn as_array(&self) -> Option<&Vec<Self>>;

    /// Try to interpret this JSON value as an array, returning a mutable reference to the underlying vector if successful.
    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>>;

    /// Try to interpret this JSON value as a string, returning a reference to the underlying string if successful.
    fn as_str(&self) -> Option<&str>;

    /// Try to interpret this JSON value as a boolean, returning the underlying boolean if successful.
    fn as_bool(&self) -> Option<bool>;

    /// Try to interpret this JSON value as an unsigned integer, returning the underlying u64 if successful.
    fn as_u64(&self) -> Option<u64>;

    /// Try to interpret this JSON value as a signed integer, returning the underlying i64 if successful.
    fn as_i64(&self) -> Option<i64>;

    /// Try to interpret this JSON value as a floating-point number, returning the underlying f64 if successful.
    fn as_f64(&self) -> Option<f64>;

    /// Try to interpret this JSON value as null, returning () if successful.
    fn as_null(&self) -> Option<()>;

    /// Return true if this JSON value is an object.
    fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// Return true if this JSON value is an array.
    fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Return true if this JSON value is a string.
    fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    /// Return true if this JSON value is a boolean.
    fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Return true if this JSON value is a number and fits into a u64.
    fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Return true if this JSON value is a number and fits into an i64.
    fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Return true if this JSON value is a number and fits into an f64.
    fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    /// Return true if this JSON value is null.
    fn is_null(&self) -> bool {
        self.as_null().is_some()
    }
}

impl JsonValue for Literal<'_> {
    type MapType = BTreeMap<String, Self>;

    fn as_object(&self) -> Option<&Self::MapType> {
        None
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::MapType> {
        None
    }

    fn as_array(&self) -> Option<&Vec<Self>> {
        None
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        None
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Literal::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Literal::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Literal::Int(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Literal::Int(n) => Some(*n),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Literal::Float(n) => Some(*n),
            Literal::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    fn as_null(&self) -> Option<()> {
        match self {
            Literal::Null => Some(()),
            _ => None,
        }
    }
}

#[cfg(feature = "serde")]
impl JsonValue for serde_json::Value {
    type MapType = serde_json::Map<String, Self>;

    fn as_object(&self) -> Option<&Self::MapType> {
        self.as_object()
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::MapType> {
        self.as_object_mut()
    }

    fn as_array(&self) -> Option<&Vec<Self>> {
        self.as_array()
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        self.as_array_mut()
    }

    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }

    fn as_bool(&self) -> Option<bool> {
        self.as_bool()
    }

    fn as_u64(&self) -> Option<u64> {
        self.as_u64()
    }

    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }

    fn as_null(&self) -> Option<()> {
        self.as_null()
    }
}
