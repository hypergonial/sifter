use std::{
    collections::{BTreeMap, HashMap},
    hash::BuildHasher,
};

use crate::Value;

/// A trait representing a JSON object map, which is a mapping from string keys to JSON values.
pub trait JsonMap<V: JsonValue>: IntoIterator<Item = (String, V)> {
    fn get(&self, key: &str) -> Option<&V>;

    fn get_mut(&mut self, key: &str) -> Option<&mut V>;

    fn insert(&mut self, key: String, value: V) -> Option<V>;

    fn contains_key(&self, key: &str) -> bool;

    fn get_key_value(&self, key: &str) -> Option<(&String, &V)>;

    fn remove(&mut self, key: &str) -> Option<V>;

    fn remove_entry(&mut self, key: &str) -> Option<(String, V)>;

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a V)>
    where
        V: 'a;

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a String, &'a mut V)>
    where
        V: 'a;
}

#[cfg(feature = "serde_json")]
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

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a serde_json::Value)>
    where
        serde_json::Value: 'a,
    {
        self.iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a String, &'a mut serde_json::Value)>
    where
        serde_json::Value: 'a,
    {
        self.iter_mut()
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

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a V)>
    where
        V: 'a,
    {
        self.iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a String, &'a mut V)>
    where
        V: 'a,
    {
        self.iter_mut()
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

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a String, &'a V)>
    where
        V: 'a,
    {
        self.iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a String, &'a mut V)>
    where
        V: 'a,
    {
        self.iter_mut()
    }
}

/// A trait representing a JSON value, which can be one of several types (object, array, string, number, boolean, or null).
///
/// Any type implementing this trait can be used as a Sosaku JSON value in bindings.
pub trait JsonValue: Sized {
    /// The type of JSON object map used by this JSON value type.
    type MapType: JsonMap<Self>;

    /// Return the corresponding JSON value for null.
    fn null() -> Self;

    /// Try to interpret this JSON value as an object, returning a reference to the underlying map if successful.
    fn as_object(&self) -> Option<&Self::MapType>;

    /// Try to interpret this JSON value as an object, returning a mutable reference to the underlying map if successful.
    fn as_object_mut(&mut self) -> Option<&mut Self::MapType>;

    /// Try to interpret this JSON value as an object, consuming it and returning the underlying map if successful.
    fn into_object(self) -> Option<Self::MapType>;

    /// Try to interpret this JSON value as an array, returning a reference to the underlying vector if successful.
    fn as_array(&self) -> Option<&Vec<Self>>;

    /// Try to interpret this JSON value as an array, returning a mutable reference to the underlying vector if successful.
    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>>;

    /// Try to interpret this JSON value as an array, consuming it and returning the underlying vector if successful.
    fn into_array(self) -> Option<Vec<Self>>;

    /// Try to interpret this JSON value as a string, consuming it and returning the underlying string if successful.
    fn into_string(self) -> Option<String>;

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

impl JsonValue for Value<'_> {
    type MapType = BTreeMap<String, Self>;

    fn null() -> Self {
        Value::Null
    }

    fn as_object(&self) -> Option<&Self::MapType> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::MapType> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    fn into_object(self) -> Option<Self::MapType> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Value::Array(vec) => Some(vec),
            _ => None,
        }
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Value::Array(vec) => Some(vec),
            _ => None,
        }
    }

    fn into_array(self) -> Option<Vec<Self>> {
        match self {
            Value::Array(vec) => Some(vec),
            _ => None,
        }
    }

    fn into_string(self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.into_owned()),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Int(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(n) => Some(*n),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    fn as_null(&self) -> Option<()> {
        match self {
            Value::Null => Some(()),
            _ => None,
        }
    }
}

#[cfg(feature = "serde_json")]
impl JsonValue for serde_json::Value {
    type MapType = serde_json::Map<String, Self>;

    fn null() -> Self {
        Self::Null
    }

    fn as_object(&self) -> Option<&Self::MapType> {
        self.as_object()
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::MapType> {
        self.as_object_mut()
    }

    fn into_object(self) -> Option<Self::MapType> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<Self>> {
        self.as_array()
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        self.as_array_mut()
    }

    fn into_array(self) -> Option<Vec<Self>> {
        match self {
            Self::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn into_string(self) -> Option<String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
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

#[cfg(feature = "serde_yaml")]
fn coerce_key_to_str(value: serde_yaml::Value) -> Result<String, String> {
    match value {
        serde_yaml::Value::String(s) => Ok(s),
        serde_yaml::Value::Number(n) => Ok(n.to_string()),
        serde_yaml::Value::Bool(b) => Ok(b.to_string()),
        serde_yaml::Value::Null => Ok("null".to_string()),
        _ => Err(format!(
            "Failed to convert YAML key '{value:?}' to string - Only string keys are supported in JSON mappings",
        )),
    }
}

/// Normalize a [`serde_yaml::Value`] into a [`serde_json::Value`], ensuring that all YAML types are
/// converted to their JSON equivalents.
///
/// Map keys are coerced into strings if possible, and an error is returned if a key cannot be converted
/// or if duplicate keys are found after coercion.
///
/// This function is useful for ensuring that YAML data can be safely used as JSON in Sosaku bindings,
/// which require JSON-compatible types.
///
/// ## Arguments
///
/// - `value`: The `serde_yaml::Value` to normalize into JSON.
///
/// ## Returns
///
/// The JSON-normalized value as a `serde_json::Value`.
///
/// ## Errors
///
/// Returns an error if a YAML mapping key cannot be coerced into a string or
/// if duplicate keys are found in a mapping after coercion.
#[cfg(feature = "serde_yaml")]
pub fn normalize_into_json(value: serde_yaml::Value) -> Result<serde_json::Value, String> {
    match value {
        serde_yaml::Value::Null => Ok(serde_json::Value::Null),
        serde_yaml::Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
        serde_yaml::Value::String(s) => Ok(serde_json::Value::String(s)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_json::Value::Number(i.into()))
            } else if let Some(u) = n.as_u64() {
                Ok(serde_json::Value::Number(u.into()))
            } else if let Some(f) = n.as_f64() {
                Ok(serde_json::Value::Number(
                    serde_json::Number::from_f64(f)
                        .ok_or_else(|| format!("Failed to convert float {f} to JSON number"))?,
                ))
            } else {
                Err(format!("Invalid YAML number: {n}"))
            }
        }
        serde_yaml::Value::Sequence(seq) => Ok(seq
            .into_iter()
            .map(normalize_into_json)
            .collect::<Result<Vec<_>, _>>()
            .map(serde_json::Value::Array)?),
        serde_yaml::Value::Mapping(map) => {
            let mut json_map = serde_json::Map::new();
            for (key, value) in map {
                let key_str = coerce_key_to_str(key)?;
                if json_map
                    .insert(key_str.clone(), normalize_into_json(value)?)
                    .is_some()
                {
                    return Err(format!("Duplicate key found in YAML mapping: {key_str}"));
                }
            }
            Ok(serde_json::Value::Object(json_map))
        }
        serde_yaml::Value::Tagged(t) => {
            // For tagged values, we can choose to ignore the tag and just convert the inner value
            normalize_into_json(t.value)
        }
    }
}
