use primitive_types::H256;
use pyo3::{exceptions::PyTypeError, prelude::*};
use revm::primitives::Address;

pub fn addr(addr: &str) -> Result<Address, PyErr> {
    addr.parse::<Address>()
        .map_err(|err| PyTypeError::new_err(err.to_string()))
}

#[allow(unused)]
pub fn h256(addr: &str) -> Result<H256, PyErr> {
    addr.parse::<H256>()
        .map_err(|err| PyTypeError::new_err(err.to_string()))
}
