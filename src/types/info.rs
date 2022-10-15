use primitive_types::H256;
use pyo3::{prelude::*, types::PyBytes};
use revm::Bytecode;
use ruint::aliases::U256;

#[pyclass]
#[derive(Debug, Default, Clone)]
pub struct AccountInfo(revm::AccountInfo);

#[pymethods]
impl AccountInfo {
    // TODO: Is there a way to avoid all this boilerplate somehow?
    #[getter]
    fn balance(_self: PyRef<'_, Self>) -> U256 {
        _self.0.balance.into()
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
        balance: Option<U256>,
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
            balance: balance.unwrap_or_default().into(),
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
