use std::collections::HashMap;

use pyo3::{PyResult, pyclass, pymethods};
use sosaku::Exp;

use crate::{errors::PySosakuError, py_types::jsonobj::PyJsonValue};

#[pyclass(from_py_object, eq, frozen, name = "Exp")]
#[derive(Debug, Clone, PartialEq)]
#[repr(transparent)]
pub struct PyExp {
    inner: Exp<'static>,
}

#[pymethods]
impl PyExp {
    /// Create a new `PyExp` from a string representation of an expression.
    ///
    /// # Errors
    ///
    /// If the provided string cannot be parsed as a valid expression,
    /// a `ValueError` will be raised with a message describing the parsing error.
    #[new]
    #[pyo3(signature = (exp, /))]
    pub fn new(exp: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Exp::try_from(exp)
                .map_err(PySosakuError::from)?
                .into_owned(),
        })
    }

    /// Evaluate the expression with the given variable bindings.
    ///
    /// # Arguments
    ///
    /// - `bindings`: A mapping containing variable bindings, where keys are variable names and values are their corresponding JSON values.
    ///
    /// # Returns
    ///
    /// The result of evaluating the expression, represented as a `PyJsonValue`.
    ///
    /// # Errors
    ///
    /// If there is an error during evaluation (e.g., undefined variable, type error), a `ValueError` will be raised with a message describing the evaluation error.
    pub fn eval(&self, bindings: HashMap<String, PyJsonValue>) -> PyResult<PyJsonValue> {
        Ok(self
            .inner
            .eval(
                &sosaku::Env::<PyJsonValue>::new()
                    .bind_multiple(bindings)
                    .build(),
            )
            .map_err(PySosakuError::from)?
            .into_owned()
            .into())
    }
}
