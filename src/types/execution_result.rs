use pyo3::{pyclass, pymethods};
use revm::primitives::ExecutionResult as RevmExecutionResult;


#[derive(Debug)]
#[pyclass(get_all)]
pub struct ExecutionResult {
    is_success: bool,
    is_halt: bool,
    reason: String,
    gas_used: u64,
    gas_refunded: u64,
    // TODO: logs: Vec<Log>,
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
        }
    }
}
