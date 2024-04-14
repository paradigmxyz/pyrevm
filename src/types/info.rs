use pyo3::{prelude::*, types::PyBytes};
use revm::primitives::{AccountInfo as RevmAccountInfo, Bytecode, KECCAK_EMPTY, U256};

#[pyclass]
#[derive(Debug, Default, Clone)]
pub struct AccountInfo(RevmAccountInfo);

#[pymethods]
impl AccountInfo {
    // TODO: Is there a way to avoid all this boilerplate somehow?
    #[getter]
    fn balance(_self: PyRef<'_, Self>) -> U256 {
        _self.0.balance
    }
    #[getter]
    fn nonce(_self: PyRef<'_, Self>) -> u64 {
        _self.0.nonce
    }
    #[getter]
    fn code(_self: PyRef<'_, Self>, py: Python<'_>) -> Option<PyObject> {
        _self
            .0
            .code
            .as_ref()
            .map(|bytecode| PyBytes::new(py, bytecode.bytes()).into())
    }

    #[getter]
    fn code_hash(_self: PyRef<'_, Self>, py: Python<'_>) -> PyObject {
        PyBytes::new(py, &_self.0.code_hash.0).into()
    }

    #[new]
    #[pyo3(signature = (balance=None, nonce=0, code_hash=None, code=None))]
    fn new(
        balance: Option<U256>,
        nonce: u64,
        code_hash: Option<&PyBytes>,
        code: Option<&PyBytes>,
    ) -> PyResult<Self> {
        let code = code.map(|bytes| Bytecode::new_raw(bytes.as_bytes().to_vec().into()));
        let code_hash = code_hash
            .and_then(|hash| hash.as_bytes().try_into().ok())
            .or_else(|| code.as_ref().map(|code| code.hash_slow()))
            .unwrap_or(KECCAK_EMPTY);
        Ok(AccountInfo(RevmAccountInfo {
            balance: balance.unwrap_or_default(),
            code_hash,
            code,
            nonce,
        }))
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self))
    }
}

impl From<RevmAccountInfo> for AccountInfo {
    fn from(env: RevmAccountInfo) -> Self {
        AccountInfo(env)
    }
}

impl From<AccountInfo> for RevmAccountInfo {
    fn from(env: AccountInfo) -> Self {
        env.0
    }
}
