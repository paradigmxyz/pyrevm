use pyo3::prelude::*;
use revm::db::{CacheDB, EmptyDB};

#[pymodule]
fn pyrevm(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<EVM>()?;
    Ok(())
}

type DB = CacheDB<EmptyDB>;

#[pyclass]
struct EVM(revm::EVM<DB>);

#[pymethods]
impl EVM {
    #[new]
    fn new() -> Self {
        let mut evm = revm::EVM::new();
        evm.database(CacheDB::new(EmptyDB()));
        EVM(evm)
    }

    fn foo(self_: PyRef<'_, Self>) -> PyResult<usize> {
        Ok(1)
    }
}
