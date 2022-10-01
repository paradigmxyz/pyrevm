use pyo3::prelude::*;
use revm::db::{CacheDB, EmptyDB};

type DB = CacheDB<EmptyDB>;

#[pyclass]
pub struct EVM(revm::EVM<DB>);

#[pymethods]
impl EVM {
    #[new]
    fn new() -> Self {
        let mut evm = revm::EVM::new();
        evm.database(CacheDB::new(EmptyDB()));
        EVM(evm)
    }

    fn foo(_self: PyRef<'_, Self>) -> PyResult<usize> {
        Ok(1)
    }
}
