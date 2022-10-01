use pyo3::prelude::*;

mod evm;
pub use evm::EVM;

#[pymodule]
fn pyrevm(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<EVM>()?;
    Ok(())
}
