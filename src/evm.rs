use crate::{types::Env, utils::addr};
use primitive_types::H160 as Address;
use pyo3::prelude::*;
use revm::{
    db::{CacheDB, EmptyDB},
    return_ok, AccountInfo, ExecutionResult, Return,
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
        let mut db = CacheDB::new(EmptyDB());
        db.insert_account_info(Address::random(), AccountInfo::default());
        evm.database(db);
        EVM(evm)
    }

    fn insert_account_info(
        mut _self: PyRefMut<'_, Self>,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        let db = _self.0.db().unwrap();
        db.insert_account_info(addr(address)?, info);

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
