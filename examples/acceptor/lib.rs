use pyo3::prelude::*;

#[pyclass]
pub struct MyClass;

#[pyfunction]
fn accept(_my_class: Py<MyClass>) {}

#[pymodule]
fn acceptor(_py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(accept, &m)?)?;
    Ok(())
}
