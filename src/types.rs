use crate::utils::{addr, u256};
use num_bigint::BigUint;
use primitive_types::H256;
use pyo3::{prelude::*, types::PyBytes};
use revm::{Bytecode, TransactTo};

#[pyclass]
#[derive(Default, Clone)]
pub struct AccountInfo(revm::AccountInfo);

#[pymethods]
impl AccountInfo {
    #[getter]
    fn balance(_self: PyRef<'_, Self>) -> PyResult<BigUint> {
        let mut bytes = [0; 32];
        _self.0.balance.to_little_endian(&mut bytes);
        let res = BigUint::from_bytes_le(&bytes);
        Ok(res)
    }

    #[new]
    #[args(nonce = "0")]
    fn new(
        balance: Option<BigUint>,
        nonce: u64,
        code_hash: Option<&PyBytes>,
        // TODO: Figure out what the best way to take in
        // bytes from pyhouu
        code: Option<&PyBytes>,
    ) -> PyResult<Self> {
        let code_hash = code_hash
            .map(|bytes| {
                let bytes = bytes.as_bytes();
                H256::from_slice(bytes)
            })
            .unwrap_or(revm::KECCAK_EMPTY);
        let code = code
            .map(|bytes| {
                let bytes = bytes.as_bytes();
                bytes.to_vec()
            })
            .map(|bytes| Bytecode::new_raw(bytes.into()));

        Ok(AccountInfo(revm::AccountInfo {
            balance: balance.map(u256).unwrap_or_default(),
            code_hash,
            code,
            nonce,
        }))
    }
}

impl From<revm::AccountInfo> for AccountInfo {
    fn from(env: revm::AccountInfo) -> Self {
        AccountInfo(env)
    }
}

impl From<AccountInfo> for revm::AccountInfo {
    fn from(env: AccountInfo) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Env(revm::Env);

#[pymethods]
impl Env {
    #[new]
    fn new(cfg: CfgEnv, block: BlockEnv, tx: TxEnv) -> Self {
        Env(revm::Env {
            cfg: cfg.into(),
            block: block.into(),
            tx: tx.into(),
        })
    }
}

impl From<revm::Env> for Env {
    fn from(env: revm::Env) -> Self {
        Env(env)
    }
}

impl From<Env> for revm::Env {
    fn from(env: Env) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Clone)]
pub struct TxEnv(revm::TxEnv);

#[pymethods]
impl TxEnv {
    #[new]
    fn new() -> Self {
        TxEnv(revm::TxEnv::default())
    }

    #[setter]
    fn caller(mut _self: PyRefMut<'_, Self>, address: &str) -> PyResult<()> {
        _self.0.caller = addr(address)?;

        Ok(())
    }

    #[setter]
    fn to(mut _self: PyRefMut<'_, Self>, address: &str) -> PyResult<()> {
        _self.0.transact_to = TransactTo::Call(
            address
                .parse::<primitive_types::H160>()
                .map_err(|err| pyo3::exceptions::PyTypeError::new_err(err.to_string()))?,
        );
        Ok(())
    }

    #[setter]
    fn value(mut _self: PyRefMut<'_, Self>, value: BigUint) -> PyResult<()> {
        _self.0.value = u256(value);
        Ok(())
    }
}

impl From<revm::TxEnv> for TxEnv {
    fn from(env: revm::TxEnv) -> Self {
        TxEnv(env)
    }
}

impl From<TxEnv> for revm::TxEnv {
    fn from(env: TxEnv) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Clone)]
pub struct BlockEnv(revm::BlockEnv);

#[pymethods]
impl BlockEnv {
    #[new]
    fn new() -> Self {
        BlockEnv(revm::BlockEnv::default())
    }
}

impl From<revm::BlockEnv> for BlockEnv {
    fn from(env: revm::BlockEnv) -> Self {
        BlockEnv(env)
    }
}

impl From<BlockEnv> for revm::BlockEnv {
    fn from(env: BlockEnv) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Clone)]
pub struct CfgEnv(revm::CfgEnv);

#[pymethods]
impl CfgEnv {
    #[new]
    fn new() -> Self {
        CfgEnv(revm::CfgEnv::default())
    }
}

impl From<revm::CfgEnv> for CfgEnv {
    fn from(env: revm::CfgEnv) -> Self {
        CfgEnv(env)
    }
}

impl From<CfgEnv> for revm::CfgEnv {
    fn from(env: CfgEnv) -> Self {
        env.0
    }
}
