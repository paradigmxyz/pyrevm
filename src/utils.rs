use primitive_types::{H160, H256};
use pyo3::{exceptions::PyTypeError, prelude::*};

pub fn addr(addr: &str) -> Result<H160, PyErr> {
    addr.parse::<H160>()
        .map_err(|err| PyTypeError::new_err(err.to_string()))
}

#[allow(unused)]
pub fn h256(addr: &str) -> Result<H256, PyErr> {
    addr.parse::<H256>()
        .map_err(|err| PyTypeError::new_err(err.to_string()))
}
