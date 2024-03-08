#![doc = include_str!("../README.md")]
#![warn(unreachable_pub)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![allow(clippy::too_many_arguments)]

// `pyo3` feature.
use ruint as _;

// Pinning `revm`.
use revm_interpreter as _;

use pyo3::prelude::*;

mod types;
pub use types::*;

mod evm;
pub use evm::EVM;

mod utils;

#[pymodule]
fn pyrevm(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<EVM>()?;

    // Types
    m.add_class::<AccountInfo>()?;
    m.add_class::<EvmOpts>()?;

    m.add_class::<Env>()?;
    m.add_class::<CfgEnv>()?;
    m.add_class::<TxEnv>()?;
    m.add_class::<BlockEnv>()?;

    Ok(())
}
