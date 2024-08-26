use pyo3::prelude::*;
use pyo3_commonize::{commonize, Commonized};

#[pyclass]
#[derive(Commonized)]
pub struct MyClass;

#[pyfunction]
fn accept(_my_class: Py<MyClass>) {}

#[pymodule]
fn acceptor(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    commonize::<MyClass>(py)?;
    m.add_function(wrap_pyfunction!(accept, &m)?)?;
    Ok(())
}
