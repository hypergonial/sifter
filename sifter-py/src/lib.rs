use pyo3::prelude::*;

pub mod py_types;

#[pymodule]
fn sifter(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<py_types::exp::PyExp>()?;
    m.add_class::<py_types::var::PyVarAccess>()?;

    Ok(())
}
