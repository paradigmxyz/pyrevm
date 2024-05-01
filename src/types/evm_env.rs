use std::default::Default;

use pyo3::types::PyTuple;
use pyo3::{pyclass, pymethods, types::PyBytes, PyObject, PyResult, Python};
use revm::primitives::{
    Address, BlobExcessGasAndPrice, BlockEnv as RevmBlockEnv, CfgEnv as RevmCfgEnv, CreateScheme,
    Env as RevmEnv, TransactTo, TxEnv as RevmTxEnv, B256, U256,
};

use crate::utils::{addr, addr_or_zero, from_pybytes};

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct Env(RevmEnv);

#[pymethods]
impl Env {
    #[new]
    fn new(cfg: Option<CfgEnv>, block: Option<BlockEnv>, tx: Option<TxEnv>) -> Self {
        Env(RevmEnv {
            cfg: cfg.unwrap_or_default().into(),
            block: block.unwrap_or_default().into(),
            tx: tx.unwrap_or_default().into(),
        })
    }

    #[getter]
    fn cfg(&self) -> CfgEnv {
        self.0.cfg.clone().into()
    }

    #[getter]
    fn block(&self) -> BlockEnv {
        self.0.block.clone().into()
    }

    #[getter]
    fn tx(&self) -> TxEnv {
        self.0.tx.clone().into()
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<RevmEnv> for Env {
    fn from(env: RevmEnv) -> Self {
        Env(env)
    }
}

impl From<Env> for RevmEnv {
    fn from(env: Env) -> Self {
        env.0
    }
}

#[pyclass]
#[derive(Debug, Default, Clone)]
pub struct TxEnv(pub RevmTxEnv);

#[pymethods]
impl TxEnv {
    #[new]
    pub fn new(
        caller: Option<&str>,
        gas_limit: Option<u64>,
        gas_price: Option<U256>,
        gas_priority_fee: Option<U256>,
        to: Option<&str>,
        value: Option<U256>,
        data: Option<Vec<u8>>,
        chain_id: Option<u64>,
        nonce: Option<u64>,
        salt: Option<U256>,
        access_list: Option<Vec<&PyTuple /*str, list[int]*/>>,
        blob_hashes: Option<Vec<&PyBytes>>,
        max_fee_per_blob_gas: Option<U256>,
    ) -> PyResult<Self> {
        Ok(TxEnv(RevmTxEnv {
            caller: addr_or_zero(caller)?,
            gas_limit: gas_limit.unwrap_or(u64::MAX),
            gas_price: gas_price.unwrap_or_default(),
            gas_priority_fee: gas_priority_fee.map(Into::into),
            transact_to: match to {
                Some(inner) => TransactTo::call(addr(inner)?),
                None => salt
                    .map(TransactTo::create2)
                    .unwrap_or_else(TransactTo::create),
            },
            value: value.unwrap_or_default(),
            data: data.unwrap_or_default().into(),
            chain_id,
            nonce,
            access_list: access_list
                .unwrap_or_default()
                .iter()
                .map(|tuple| {
                    Ok((
                        addr(tuple.get_item(0)?.extract()?)?,
                        tuple.get_item(1)?.extract::<Vec<U256>>()?,
                    ))
                })
                .collect::<PyResult<Vec<(Address, Vec<U256>)>>>()?,
            blob_hashes: blob_hashes
                .unwrap_or_default()
                .iter()
                .map(|b| from_pybytes(b))
                .collect::<PyResult<Vec<B256>>>()?,
            max_fee_per_blob_gas,
        }))
    }

    #[getter]
    fn caller(&self) -> String {
        self.0.caller.to_string()
    }

    #[getter]
    fn gas_limit(&self) -> u64 {
        self.0.gas_limit
    }

    #[getter]
    fn gas_price(&self) -> U256 {
        self.0.gas_price
    }

    #[getter]
    fn gas_priority_fee(&self) -> Option<U256> {
        self.0.gas_priority_fee.map(Into::into)
    }

    #[getter]
    fn to(&self) -> Option<String> {
        match &self.0.transact_to {
            TransactTo::Call(address) => Some(format!("{:?}", address)),
            TransactTo::Create(_) => None,
        }
    }

    #[getter]
    fn value(&self) -> U256 {
        self.0.value
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyObject {
        PyBytes::new(py, self.0.data.as_ref()).into()
    }

    #[getter]
    fn chain_id(&self) -> Option<u64> {
        self.0.chain_id
    }

    #[getter]
    fn nonce(&self) -> Option<u64> {
        self.0.nonce
    }

    #[getter]
    fn salt(&self) -> Option<U256> {
        if let TransactTo::Create(CreateScheme::Create2 { salt }) = self.0.transact_to {
            return Some(salt);
        }
        None
    }

    #[getter]
    fn access_list(&self) -> Vec<(String, Vec<U256>)> {
        self.0
            .access_list
            .iter()
            .map(|(a, b)| (a.to_string(), b.clone()))
            .collect()
    }

    #[getter]
    fn blob_hashes(&self, py: Python<'_>) -> Vec<PyObject> {
        self.0
            .blob_hashes
            .iter()
            .map(|i| PyBytes::new(py, i.0.as_ref()).into())
            .collect()
    }

    #[getter]
    fn max_fee_per_blob_gas(&self) -> Option<U256> {
        self.0.max_fee_per_blob_gas
    }

    #[setter]
    fn set_blob_hashes(&mut self, blob_hashes: Vec<&PyBytes>) -> PyResult<()> {
        self.0.blob_hashes = blob_hashes
            .iter()
            .map(|b| from_pybytes(b))
            .collect::<PyResult<Vec<B256>>>()?;
        Ok(())
    }

    #[setter]
    fn set_max_fee_per_blob_gas(&mut self, max_fee_per_blob_gas: Option<U256>) {
        self.0.max_fee_per_blob_gas = max_fee_per_blob_gas;
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<TxEnv> for RevmTxEnv {
    fn from(env: TxEnv) -> Self {
        env.0
    }
}

impl From<RevmTxEnv> for TxEnv {
    fn from(val: RevmTxEnv) -> Self {
        TxEnv(val)
    }
}

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct BlockEnv(RevmBlockEnv);

#[pymethods]
impl BlockEnv {
    #[new]
    fn new(
        number: Option<U256>,
        coinbase: Option<&str>,
        timestamp: Option<U256>,
        difficulty: Option<U256>,
        prevrandao: Option<&PyBytes>,
        basefee: Option<U256>,
        gas_limit: Option<U256>,
        excess_blob_gas: Option<u64>,
    ) -> PyResult<Self> {
        Ok(BlockEnv(RevmBlockEnv {
            number: number.unwrap_or_default(),
            coinbase: addr_or_zero(coinbase)?,
            timestamp: timestamp.unwrap_or(U256::from(1)),
            difficulty: difficulty.unwrap_or_default(),
            prevrandao: Some(match prevrandao {
                Some(b) => from_pybytes(b)?,
                None => B256::ZERO,
            }),
            basefee: basefee.unwrap_or_default(),
            gas_limit: gas_limit.unwrap_or_else(|| U256::from(u64::MAX)),
            blob_excess_gas_and_price: Some(BlobExcessGasAndPrice::new(
                excess_blob_gas.unwrap_or(0),
            )),
        }))
    }

    #[getter]
    fn number(&self) -> U256 {
        self.0.number
    }

    #[getter]
    fn coinbase(&self) -> String {
        self.0.coinbase.to_string()
    }

    #[getter]
    fn timestamp(&self) -> U256 {
        self.0.timestamp
    }

    #[getter]
    fn difficulty(&self) -> U256 {
        self.0.difficulty
    }

    #[getter]
    fn prevrandao(&self, py: Python<'_>) -> Option<PyObject> {
        self.0
            .prevrandao
            .map(|i| PyBytes::new(py, i.0.as_ref()).into())
    }

    #[getter]
    fn basefee(&self) -> U256 {
        self.0.basefee
    }

    #[getter]
    fn gas_limit(&self) -> U256 {
        self.0.gas_limit
    }

    #[getter]
    fn excess_blob_gas(&self) -> Option<u64> {
        self.0
            .blob_excess_gas_and_price
            .clone()
            .map(|i| i.excess_blob_gas)
    }

    #[getter]
    fn blob_gasprice(&self) -> Option<u128> {
        self.0
            .blob_excess_gas_and_price
            .clone()
            .map(|i| i.blob_gasprice)
    }

    #[setter]
    fn set_number(&mut self, number: U256) {
        self.0.number = number;
    }

    #[setter]
    fn set_timestamp(&mut self, timestamp: U256) {
        self.0.timestamp = timestamp;
    }

    #[setter]
    fn set_excess_blob_gas(&mut self, excess_blob_gas: Option<u64>) {
        self.0.blob_excess_gas_and_price = excess_blob_gas.map(BlobExcessGasAndPrice::new);
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<BlockEnv> for RevmBlockEnv {
    fn from(env: BlockEnv) -> Self {
        env.0
    }
}

impl From<RevmBlockEnv> for BlockEnv {
    fn from(val: RevmBlockEnv) -> Self {
        BlockEnv(val)
    }
}

#[pyclass]
#[derive(Default, Clone, Debug)]
pub struct CfgEnv(RevmCfgEnv);

#[pymethods]
impl CfgEnv {
    #[new]
    fn new() -> Self {
        CfgEnv(RevmCfgEnv::default())
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<CfgEnv> for RevmCfgEnv {
    fn from(env: CfgEnv) -> Self {
        env.0
    }
}

impl From<RevmCfgEnv> for CfgEnv {
    fn from(val: RevmCfgEnv) -> Self {
        CfgEnv(val)
    }
}
