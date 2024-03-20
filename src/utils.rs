use pyo3::{exceptions::PyTypeError, prelude::*};
use revm::primitives::{Address, HashMap as RevmHashMap};
use std::fmt::Debug;
use pyo3::exceptions::PyRuntimeError;
use revm::db::DbAccount;
use crate::types::PyDB;

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

pub(crate) fn to_hashmap(map: &RevmHashMap<Address, DbAccount>) -> PyDB {
    map.iter().map(
        |(address, db_acc)| (address.to_string(), db_acc.info.clone().into())
    ).collect()
}
