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

#[pymethods]
impl EvmOpts {
    #[new]
    fn new(env: Option<Env>, fork_url: Option<String>) -> Self {
        Self {
            fork_url,
            ..Default::default()
        }
    }
}
