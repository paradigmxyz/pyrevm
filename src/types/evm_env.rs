use crate::utils::{addr, addr_or_zero};
use pyo3::{exceptions::PyTypeError, pyclass, pymethods, PyObject, PyResult, Python, types::{PyBytes}};
use revm::primitives::{BlobExcessGasAndPrice, BlockEnv as RevmBlockEnv, CfgEnv as RevmCfgEnv, Env as RevmEnv, TransactTo, TxEnv as RevmTxEnv, B256, U256, CreateScheme};

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
    ) -> PyResult<Self> {
        Ok(TxEnv(RevmTxEnv {
            caller: addr_or_zero(caller)?,
            gas_limit: gas_limit.unwrap_or(u64::MAX),
            gas_price: gas_price.unwrap_or_default(),
            gas_priority_fee: gas_priority_fee.map(Into::into),
            transact_to: match to {
                Some(inner) => TransactTo::call(addr(inner)?),
                None => if salt.is_some() { TransactTo::create2(salt.unwrap()) } else { TransactTo::create() },
            },
            value: value.unwrap_or_default(),
            data: data.unwrap_or_default().into(),
            chain_id,
            nonce,
            // TODO: Add access list.
            ..Default::default()
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
        PyBytes::new(py, &self.0.data.to_vec()).into()
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
        if let TransactTo::Create(scheme) = self.0.transact_to {
            if let CreateScheme::Create2 { salt } = scheme {
                return Some(salt);
            }
        }
        None
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

impl Into<TxEnv> for RevmTxEnv {
    fn into(self) -> TxEnv {
        TxEnv(self)
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
        let prevrandao = match prevrandao {
            Some(b) => {
                B256::try_from(b.as_bytes()).map_err(|e| PyTypeError::new_err(e.to_string()))?
            }
            None => B256::ZERO,
        };
        Ok(BlockEnv(RevmBlockEnv {
            number: number.unwrap_or_default(),
            coinbase: addr_or_zero(coinbase)?,
            timestamp: timestamp.unwrap_or(U256::from(1)),
            difficulty: difficulty.unwrap_or_default(),
            prevrandao: Some(prevrandao),
            basefee: basefee.unwrap_or_default(),
            gas_limit: gas_limit.unwrap_or_else(|| U256::from(u64::MAX)),
            blob_excess_gas_and_price: Some(BlobExcessGasAndPrice::new(
                excess_blob_gas.unwrap_or(0),
            )),
        }))
    }

    #[getter]
    fn number(&self) -> U256  {
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
    fn prevrandao(&self) -> Option<[u8; 32]> {
        self.0.prevrandao.map(|i| i.0)
    }

    #[getter]
    fn basefee(&self) -> U256 {
        self.0.basefee
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

impl Into<BlockEnv> for RevmBlockEnv {
    fn into(self) -> BlockEnv {
        BlockEnv(self)
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

impl Into<CfgEnv> for RevmCfgEnv {
    fn into(self) -> CfgEnv {
        CfgEnv(self)
    }
}
