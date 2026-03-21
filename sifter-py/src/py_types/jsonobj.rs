use std::collections::BTreeMap;

use pyo3::{
    Borrowed, Bound, FromPyObject, IntoPyObject, IntoPyObjectExt, PyAny, PyResult,
    types::{PyAnyMethods, PyMapping, PyMappingMethods, PySequenceMethods},
};
use sifter::JsonObject;

#[derive(Debug, Clone, PartialEq)]
pub enum PyJsonValue {
    Null,
    Bool(bool),
    UInt(u64),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl<'a, 'py> FromPyObject<'a, 'py> for PyJsonValue {
    type Error = pyo3::PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if ob.is_none() {
            Ok(Self::Null)
        } else if let Ok(b) = ob.extract::<bool>() {
            Ok(Self::Bool(b))
        } else if let Ok(i) = ob.extract::<u64>() {
            Ok(Self::UInt(i))
        } else if let Ok(u) = ob.extract::<i64>() {
            Ok(Self::Int(u))
        } else if let Ok(f) = ob.extract::<f64>() {
            Ok(Self::Float(f))
        } else if let Ok(s) = ob.extract::<String>() {
            Ok(Self::String(s))
        } else if let Ok(seq) = ob.cast::<pyo3::types::PySequence>() {
            let mut vec = Vec::with_capacity(seq.len().unwrap_or(0));
            for item in seq.try_iter()? {
                vec.push(item?.extract()?);
            }
            Ok(Self::Array(vec))
        } else if let Ok(mapping) = ob.cast::<PyMapping>() {
            let mut map = BTreeMap::new();
            for key in mapping.keys()? {
                let key_str: String = key.extract()?;
                let value: Self = mapping.get_item(key)?.extract()?;
                map.insert(key_str, value);
            }
            Ok(Self::Object(map))
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "Unsupported type for JSON conversion",
            ))
        }
    }
}

impl<'py> IntoPyObject<'py> for PyJsonValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = pyo3::PyErr;

    fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            Self::Null => Ok(py.None().bind(py).clone()),
            Self::Bool(b) => Ok(b.into_bound_py_any(py)?),
            Self::UInt(u) => Ok(u.into_bound_py_any(py)?),
            Self::Int(i) => Ok(i.into_bound_py_any(py)?),
            Self::Float(f) => Ok(f.into_bound_py_any(py)?),
            Self::String(s) => Ok(s.into_bound_py_any(py)?),
            Self::Array(vec) => Ok(vec.into_bound_py_any(py)?),
            Self::Object(map) => Ok(map.into_bound_py_any(py)?),
        }
    }
}

impl JsonObject for PyJsonValue {
    type MapType = BTreeMap<String, Self>;

    fn as_object(&self) -> Option<&Self::MapType> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::MapType> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Array(vec) => Some(vec),
            _ => None,
        }
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::Array(vec) => Some(vec),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::UInt(u) => Some(*u),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_null(&self) -> Option<()> {
        match self {
            Self::Null => Some(()),
            _ => None,
        }
    }
}
