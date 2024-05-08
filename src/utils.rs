use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyBytes;
use pyo3::{exceptions::PyTypeError, prelude::*};
use revm::precompile::B256;
use revm::primitives::{fake_exponential as revm_fake_exponential, Address};
use std::fmt;

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
pub(crate) fn pyerr<T: fmt::Debug>(err: T) -> PyErr {
    PyRuntimeError::new_err(format!("{err:?}"))
}

pub(crate) fn from_pybytes(b: &PyBytes) -> PyResult<B256> {
    B256::try_from(b.as_bytes()).map_err(|e| PyTypeError::new_err(e.to_string()))
}

#[pyfunction]
pub fn fake_exponential(factor: u64, numerator: u64, denominator: u64) -> u128 {
    revm_fake_exponential(factor, numerator, denominator)
}
