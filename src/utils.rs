use num_bigint::BigUint;
use primitive_types::{H160, H256, U256};
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

pub fn u256(value: BigUint) -> U256 {
    U256::from_little_endian(&value.to_bytes_le())
}

pub fn uint(value: U256) -> BigUint {
    let mut bytes = [0; 32];
    value.to_little_endian(&mut bytes);
    BigUint::from_bytes_le(&bytes)
}
