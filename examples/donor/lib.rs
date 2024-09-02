use acceptor::MyClass;
use pyo3::prelude::*;
use pyo3_commonize::commonize;
//
//

#[pyfunction]
fn generate() -> MyClass {
    MyClass
}

#[pymodule]
fn donor(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    commonize::<MyClass>(py)?;
    m.add_function(wrap_pyfunction!(generate, &m)?)?;
    Ok(())
}
//hello
//hello
