use pyo3::{exceptions::PyTypeError, prelude::*};
use revm::primitives::Address;

pub fn addr(addr: &str) -> Result<Address, PyErr> {
    addr.parse::<Address>()
        .map_err(|err| PyTypeError::new_err(err.to_string()))
}
