use super::Env;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct EvmOpts {
    pub env: Env,
    pub fork_url: Option<String>,
    pub fork_block_number: Option<u64>,
    pub gas_limit: u64,
    pub tracing: bool,
}
