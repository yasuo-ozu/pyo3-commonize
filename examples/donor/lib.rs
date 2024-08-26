use acceptor::MyClass;
use pyo3::prelude::*;

#[pyfunction]
fn generate() -> MyClass {
    MyClass
}

#[pymodule]
fn donor(_py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate, &m)?)?;
    Ok(())
}
