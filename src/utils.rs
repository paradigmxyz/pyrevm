use std::fmt::Debug;

use pyo3::{exceptions::PyTypeError, prelude::*};
use pyo3::exceptions::PyRuntimeError;
use revm::primitives::Address;

pub(crate) fn addr(s: &str) -> Result<Address, PyErr> {
    s.parse::<Address>()
        .map_err(|err| PyTypeError::new_err(err.to_string()))
}

pub(crate) fn addr_or_zero(s: Option<&str>) -> Result<Address, PyErr> {
    match s {
        Some(s) => addr(s),
        None => Ok(Address::ZERO),
    }
}

/// Convert a Rust error into a Python error.
pub(crate) fn pyerr<T: Debug>(err: T) -> PyErr {
    PyRuntimeError::new_err(format!("{:?}", err))
}
