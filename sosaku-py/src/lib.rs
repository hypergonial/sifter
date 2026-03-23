use pyo3::prelude::*;

pub mod errors;
pub mod py_types;

#[pymodule]
fn sosaku(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<py_types::exp::PyExp>()?;
    m.add_class::<py_types::var::PyVarAccess>()?;

    Ok(())
}
