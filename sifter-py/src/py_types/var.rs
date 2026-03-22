use std::collections::HashMap;

use pyo3::{PyResult, exceptions::PyValueError, pyclass, pymethods};
use sifter::VarAccess;

use crate::py_types::jsonobj::PyJsonValue;

#[pyclass(from_py_object, eq, frozen, hash, name = "VarAccess")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PyVarAccess {
    inner: VarAccess,
}

#[pymethods]
#[allow(clippy::needless_pass_by_value)]
impl PyVarAccess {
    /// Create a new `PyVarAccess` from a string representation of a variable access.
    ///
    /// # Arguments
    ///
    /// - `accessor`: A string slice representing the accessor for the variable to access.
    ///
    /// # Errors
    ///
    /// If the provided string cannot be parsed as a valid variable access, a `ValueError` will be raised with a message describing the parsing error.
    #[new]
    #[pyo3(signature = (accessor, /))]
    pub fn new(accessor: &str) -> PyResult<Self> {
        Ok(Self {
            inner: VarAccess::try_from(accessor)
                .map_err(|e| PyValueError::new_err(e.to_string()))?,
        })
    }

    /// Access the variable specified by this `PyVarAccess` from the given JSON value.
    ///
    /// # Arguments
    ///
    /// - `value`: The JSON value from which to access the variable specified by this `PyVarAccess`.
    ///
    /// # Returns
    ///
    /// - A `PyJsonValue` representing the value accessed from the provided JSON value according to the variable access specified by this `PyVarAccess`.
    ///
    /// # Errors
    ///
    /// - If there is an error during the variable access (e.g., variable not found, type mismatch), a `ValueError` will be raised with a message describing the error.
    #[pyo3(signature = (value, /))]
    pub fn access(&self, value: PyJsonValue) -> PyResult<PyJsonValue> {
        Ok(self
            .inner
            .access(&value)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into())
    }

    /// Access the variable specified by this `PyVarAccess` from the given variable bindings.
    ///
    /// # Arguments
    ///
    /// - `bindings`: A mapping containing variable bindings, where keys are variable names and values are their corresponding JSON values.
    ///
    /// # Returns
    ///
    /// - A `PyJsonValue` representing the value accessed from the provided variable bindings according to the variable access specified by this `PyVarAccess`.
    ///
    /// # Errors
    ///
    /// - If there is an error during the variable access (e.g., variable not found, type mismatch), a `ValueError` will be raised with a message describing the error.
    #[pyo3(signature = (bindings, /))]
    pub fn access_from_bindings(
        &self,
        bindings: HashMap<String, PyJsonValue>,
    ) -> PyResult<PyJsonValue> {
        Ok(self
            .inner
            .access_from_bindings(&sifter::Env::new().bind_multiple(bindings).build())
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into())
    }
}
