use crate::{
    types::{AccountInfo, Env},
    utils::addr,
};
use pyo3::prelude::*;
use revm::{
    db::{CacheDB, DatabaseRef, EmptyDB},
    return_ok, Return,
};

type DB = CacheDB<EmptyDB>;

#[pyclass]
pub struct EVM(revm::EVM<DB>);

impl EVM {
    pub fn db(&self) -> &DB {
        self.0.db.as_ref().unwrap()
    }
}

#[pymethods]
impl EVM {
    #[new]
    fn new() -> Self {
        let mut evm = revm::EVM::new();
        let db = CacheDB::new(EmptyDB());
        evm.database(db);
        EVM(evm)
    }

    /// Inserts the provided account information in the database at
    /// the specified address.
    fn basic(_self: PyRef<'_, Self>, address: &str) -> PyResult<Option<AccountInfo>> {
        let db = _self.0.db.as_ref().unwrap();
        let acc = db.basic(addr(address)?)?;
        Ok(acc.map(Into::into))
    }

    /// Inserts the provided account information in the database at
    /// the specified address.
    fn insert_account_info(
        mut _self: PyRefMut<'_, Self>,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        let db = _self.0.db().unwrap();
        db.insert_account_info(addr(address)?, info.into());

        Ok(())
    }

    #[setter]
    fn env(mut _self: PyRefMut<'_, Self>, env: Env) -> PyResult<()> {
        _self.0.env = env.into();

        Ok(())
    }

    fn transact_commit(mut _self: PyRefMut<'_, Self>) -> PyResult<u64> {
        let res = _self.0.transact_commit();
        if !matches!(res.exit_reason, return_ok!()) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "{:?}",
                res.exit_reason
            )));
        }

        Ok(res.gas_used)
    }

    fn dump(_self: PyRef<'_, Self>) -> PyResult<()> {
        println!("State");
        println!("{:?}", _self.0.env);
        println!("{:?}", _self.db());

        Ok(())
    }

    fn foo(_self: PyRef<'_, Self>) -> PyResult<usize> {
        Ok(1)
    }
}
