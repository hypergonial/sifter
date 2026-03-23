use std::{error::Error, fmt::Display};

use pyo3::{
    Bound, IntoPyObject, IntoPyObjectExt, PyAny, Python,
    exceptions::{PyNameError, PyTypeError, PyValueError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PySosakuError {
    inner: sosaku::Error,
}

impl<'py> IntoPyObject<'py> for PySosakuError {
    type Target = PyAny;

    type Output = Bound<'py, Self::Target>;

    type Error = pyo3::PyErr;

    fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.inner {
            sosaku::Error::Parse(e) => {
                Ok(PyValueError::new_err(e.to_string()).into_bound_py_any(py)?)
            }
            sosaku::Error::Eval(e) => match e {
                sosaku::EvalError::FnCallError(fne) => Self {
                    inner: sosaku::Error::from(fne.reason.as_ref().clone()),
                }
                .into_pyobject(py),
                sosaku::EvalError::VarAccess(va) => {
                    Ok(PyNameError::new_err(va.to_string()).into_bound_py_any(py)?)
                }
                sosaku::EvalError::FunctionNotFound { .. } => {
                    Ok(PyNameError::new_err(e.to_string()).into_bound_py_any(py)?)
                }
                sosaku::EvalError::TypeError { message } => {
                    Ok(PyTypeError::new_err(message).into_bound_py_any(py)?)
                }
                sosaku::EvalError::ValueError { message }
                | sosaku::EvalError::RegexError { message } => {
                    Ok(PyValueError::new_err(message).into_bound_py_any(py)?)
                }
                sosaku::EvalError::ArgumentCount { expected, got } => Ok(PyValueError::new_err(
                    format!("Expected {expected} arguments, got {got}"),
                )
                .into_bound_py_any(py)?),
            },
        }
    }
}

impl<T: Into<sosaku::Error>> From<T> for PySosakuError {
    fn from(value: T) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl From<PySosakuError> for pyo3::PyErr {
    fn from(error: PySosakuError) -> Self {
        Python::attach(|py| {
            Self::from_value(
                error
                    .into_pyobject(py)
                    .expect("Failed to convert PySosakuError to PyErr"),
            )
        })
    }
}

impl Display for PySosakuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Error for PySosakuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}
