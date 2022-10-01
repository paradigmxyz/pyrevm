use crate::utils::{addr, u256, uint};
use num_bigint::BigUint;
use primitive_types::{H256, U256};
use pyo3::{prelude::*, types::PyBytes};
use revm::{Bytecode, TransactTo};

#[pyclass]
#[derive(Debug, Default, Clone)]
pub struct AccountInfo(revm::AccountInfo);

#[pymethods]
impl AccountInfo {
    // TODO: Is there a way to avoid all this boilerplate somehow?
    #[getter]
    fn balance(_self: PyRef<'_, Self>) -> BigUint {
        uint(_self.0.balance)
    }
    #[getter]
    fn nonce(_self: PyRef<'_, Self>) -> u64 {
        _self.0.nonce
    }
    #[getter]
    fn code(_self: PyRef<'_, Self>) -> Vec<u8> {
        _self
            .0
            .code
            .as_ref()
            .map(|x| x.bytes().to_vec())
            .unwrap_or_default()
    }
    #[getter]
    fn code_hash(_self: PyRef<'_, Self>) -> [u8; 32] {
        _self.0.code_hash.to_fixed_bytes()
    }

    #[new]
    #[args(nonce = "0")]
    fn new(
        balance: Option<BigUint>,
        nonce: u64,
        code_hash: Option<&PyBytes>,
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

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
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
    fn new(cfg: Option<CfgEnv>, block: Option<BlockEnv>, tx: Option<TxEnv>) -> Self {
        Env(revm::Env {
            cfg: cfg.unwrap_or_default().into(),
            block: block.unwrap_or_default().into(),
            tx: tx.unwrap_or_default().into(),
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
#[derive(Debug, Default, Clone)]
pub struct TxEnv(revm::TxEnv);

#[pymethods]
impl TxEnv {
    #[new]
    fn new(
        caller: Option<&str>,
        gas_limit: Option<u64>,
        gas_price: Option<BigUint>,
        gas_priority_fee: Option<BigUint>,
        to: Option<&str>,
        value: Option<BigUint>,
        data: Option<Vec<u8>>,
        chain_id: Option<u64>,
        nonce: Option<u64>,
    ) -> PyResult<Self> {
        Ok(TxEnv(revm::TxEnv {
            caller: addr(caller.unwrap_or_default())?,
            gas_limit: gas_limit.unwrap_or(u64::MAX),
            gas_price: u256(gas_price.unwrap_or_default()),
            gas_priority_fee: gas_priority_fee.map(u256),
            transact_to: match to {
                Some(inner) => TransactTo::Call(addr(inner)?),
                // TODO: Figure out how to integrate CREATE2 here
                None => TransactTo::Create(revm::CreateScheme::Create),
            },
            value: u256(value.unwrap_or_default()),
            data: data.unwrap_or_default().into(),
            chain_id,
            nonce,
            // TODO: Add access list.
            ..Default::default()
        }))
    }
}

impl From<TxEnv> for revm::TxEnv {
    fn from(env: TxEnv) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct BlockEnv(revm::BlockEnv);

#[pymethods]
impl BlockEnv {
    #[new]
    fn new(
        number: Option<BigUint>,
        coinbase: Option<&str>,
        timestamp: Option<BigUint>,
        difficulty: Option<BigUint>,
        basefee: Option<BigUint>,
        gas_limit: Option<BigUint>,
    ) -> PyResult<Self> {
        Ok(BlockEnv(revm::BlockEnv {
            number: u256(number.unwrap_or_default()),
            coinbase: addr(coinbase.unwrap_or("0x0000000000000000000000000000000000000000"))?,
            timestamp: if let Some(timestamp) = timestamp {
                u256(timestamp)
            } else {
                U256::from(1)
            },
            difficulty: u256(difficulty.unwrap_or_default()),
            basefee: u256(basefee.unwrap_or_default()),
            gas_limit: u256(gas_limit.unwrap_or_else(|| u64::MAX.into())),
        }))
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<BlockEnv> for revm::BlockEnv {
    fn from(env: BlockEnv) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Default, Clone, Debug)]
pub struct CfgEnv(revm::CfgEnv);

#[pymethods]
impl CfgEnv {
    #[new]
    fn new() -> Self {
        CfgEnv(revm::CfgEnv::default())
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<CfgEnv> for revm::CfgEnv {
    fn from(env: CfgEnv) -> Self {
        env.0
    }
}
