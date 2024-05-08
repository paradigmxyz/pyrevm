#![doc = include_str!("../README.md")]
#![warn(unreachable_pub)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![allow(non_local_definitions)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![allow(clippy::too_many_arguments)]

use pyo3::prelude::*;

mod database;
mod empty_db_wrapper;
mod evm;
mod executor;
mod pystdout;
mod types;
mod utils;

pub use evm::EVM;
pub use types::*;
pub use utils::fake_exponential;

#[pymodule]
fn pyrevm(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<EVM>()?;

    // Types
    m.add_class::<AccountInfo>()?;
    m.add_class::<Env>()?;
    m.add_class::<CfgEnv>()?;
    m.add_class::<TxEnv>()?;
    m.add_class::<BlockEnv>()?;
    m.add_class::<ExecutionResult>()?;
    m.add_class::<Log>()?;
    m.add_class::<JournalCheckpoint>()?;
    m.add_function(wrap_pyfunction!(fake_exponential, m)?)?;

    Ok(())
}
