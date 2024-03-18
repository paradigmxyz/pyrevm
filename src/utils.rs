use pyo3::{exceptions::PyTypeError, prelude::*};
use revm::primitives::{Address, State};
use std::fmt::Debug;
use pyo3::exceptions::PyRuntimeError;
use std::collections::HashMap;
use crate::AccountInfo;

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

pub(crate) fn pydict(res: State) -> HashMap<String, AccountInfo> {
    res.iter().map(
        |(address, db_acc)| (address.to_string(), db_acc.info.clone().into())
    ).collect()
}

/// Convert a Rust error into a Python error.
pub(crate) fn pyerr<T: Debug>(err: T) -> PyErr {
    PyRuntimeError::new_err(format!("{:?}", err))
}
