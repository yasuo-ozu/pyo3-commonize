# pyo3-commonize [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![GitHub Actions]][actions]

[Latest Version]: https://img.shields.io/crates/v/pyo3-commonize.svg
[crates.io]: https://crates.io/crates/pyo3-commonize
[Documentation]: https://img.shields.io/docsrs/pyo3-commonize
[docs.rs]: https://docs.rs/pyo3-commonize/latest/pyo3-commonize/
[GitHub Actions]: https://github.com/yasuo-ozu/pyo3-commonize/actions/workflows/rust.yml/badge.svg
[actions]: https://github.com/yasuo-ozu/pyo3-commonize/actions/workflows/rust.yml

Using this crate, the classes defined using #[pyclass] can be passed between multiple Python native extensions built with PyO3.

## ABI safetyness

When using this crate, it is essential to pay strict attention to ABI compatibility. To define ABI-compatible classes, it is recommended to use crates like abi_stable. (In the future, this crate may require the implementation of `abi_stable::StableAbi`.)

To mitigate this issue, pyo3-commonize checks the following points: the last modified date of the source code for the specified crate and all of its dependency crates, as well as the build environment (target and host versions, Rust flags, optimization levels, etc.).

## Quick setup

Example to pass `MyClass` class defined in `acceptor` crate from `donor` to `acceptor`.

### `acceptor` crate

- in `Cargo.toml`

```toml
[package]
name = "acceptor"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["rlib", "cdylib"]
[dependencies]
pyo3-commonize = "0.1.0"
pyo3 = "*"  # pyo3-commonize fixes pyo3 version
```

- in `pyproject.toml`

```toml
[build-system]
requires = ["poetry-core>=1.0.0", "maturin>=1.0,<2.0"]
build-backend = "maturin"
[project]
name = "acceptor"
version = "0.1.0"
license = { text = "MIT" }
```

- in `src/lib.rs`

```rust
use pyo3::prelude::*;
use pyo3_commonize::{commonize, Commonized};

#[pyclass]
#[derive(Commonized)]
pub struct MyClass;

#[pyfunction]
fn accept(_my_class: Py<MyClass>) {}

#[pymodule]
fn acceptor(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    // This function should be called at first
    commonize::<MyClass>(py)?;
    m.add_function(wrap_pyfunction!(accept, &m)?)?;
    Ok(())
}
```

## `donor` crate

- in `Cargo.toml`

```toml
[package]
name = "donor"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["rlib", "cdylib"]
[dependencies]
pyo3-commonize = "0.1.0"
pyo3 = "*"  # pyo3-commonize fixes pyo3 version
acceptor = { path = "<path>" }
```

- in `pyproject.toml`

```toml
[build-system]
requires = ["poetry-core>=1.0.0", "maturin>=1.0,<2.0"]
build-backend = "maturin"
[project]
name = "donor"
version = "0.1.0"
license = { text = "MIT" }
```

- in `src/lib.rs`

```rust
use acceptor::MyClass;
use pyo3::prelude::*;
use pyo3_commonize::commonize;

#[pyfunction]
fn generate() -> MyClass { MyClass }

#[pymodule]
fn donor(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    commonize::<MyClass>(py)?;
    m.add_function(wrap_pyfunction!(generate, &m)?)?;
    Ok(())
}
```
