use std::collections::HashMap;

use pyo3::{PyResult, exceptions::PyValueError, pyclass, pymethods};
use sifter::Exp;

use crate::py_types::jsonobj::PyJsonValue;

#[pyclass(from_py_object)]
#[derive(Debug, Clone, PartialEq)]
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
                .map_err(|e| PyValueError::new_err(e.to_string()))?
                .into_owned(),
        })
    }

    /// Evaluate the expression with the given variable bindings.
    ///
    /// # Arguments
    ///
    /// - `env`: A mapping containing variable bindings, where keys are variable names and values are their corresponding JSON values.
    ///
    /// # Returns
    ///
    /// The result of evaluating the expression, represented as a `PyJsonValue`.
    ///
    /// # Errors
    ///
    /// If there is an error during evaluation (e.g., undefined variable, type error), a `ValueError` will be raised with a message describing the evaluation error.
    pub fn eval(&self, env: HashMap<String, PyJsonValue>) -> PyResult<PyJsonValue> {
        let _inner_env = sifter::Env::new().bind_multiple(env).build();
        //let res = self.inner.eval(&inner_env);

        todo!()
    }
}
