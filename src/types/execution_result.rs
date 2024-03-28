use pyo3::types::PyBytes;
use pyo3::{pyclass, pymethods, PyObject, Python};
use revm::primitives::{ExecutionResult as RevmExecutionResult, Log as RevmLog};

#[derive(Debug, Clone, Hash)]
#[pyclass]
pub struct Log(RevmLog);

#[pymethods]
impl Log {
    #[getter]
    fn address(&self) -> String {
        self.0.address.to_string()
    }

    #[getter]
    fn topics(&self) -> Vec<String> {
        self.0.topics().iter().map(|x| x.to_string()).collect()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> (Vec<PyObject>, PyObject) {
        let topics = self
            .0
            .data
            .topics()
            .iter()
            .map(|t| PyBytes::new(py, &t.0).into())
            .collect();
        let data = PyBytes::new(py, &self.0.data.data).into();
        (topics, data)
    }
}

/// Result of a transaction execution.
#[derive(Debug, Clone, Hash)]
#[pyclass(get_all)]
pub struct ExecutionResult {
    is_success: bool,
    is_halt: bool,
    reason: String,
    gas_used: u64,
    gas_refunded: u64,
    logs: Vec<Log>,
}

#[pymethods]
impl ExecutionResult {}

impl From<RevmExecutionResult> for ExecutionResult {
    fn from(result: RevmExecutionResult) -> Self {
        ExecutionResult {
            is_success: result.is_success(),
            is_halt: result.is_halt(),
            reason: match result {
                RevmExecutionResult::Success { reason, .. } => format!("{:?}", reason),
                RevmExecutionResult::Revert { .. } => String::from("Revert"),
                RevmExecutionResult::Halt { reason, .. } => format!("{:?}", reason),
            },
            gas_used: match result {
                RevmExecutionResult::Success { gas_used, .. } => gas_used,
                RevmExecutionResult::Revert { gas_used, .. } => gas_used,
                RevmExecutionResult::Halt { gas_used, .. } => gas_used,
            },
            gas_refunded: match result {
                RevmExecutionResult::Success { gas_refunded, .. } => gas_refunded,
                _ => u64::default(),
            },
            logs: match result {
                RevmExecutionResult::Success { logs, .. } => logs.into_iter().map(Log).collect(),
                _ => Vec::new(),
            },
        }
    }
}

impl From<RevmLog> for Log {
    fn from(env: RevmLog) -> Self {
        Log(env)
    }
}

impl From<Log> for RevmLog {
    fn from(env: Log) -> Self {
        env.0
    }
}
