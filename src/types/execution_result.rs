use pyo3::{pyclass, pymethods};
use revm::primitives::{ExecutionResult as RevmExecutionResult, HaltReason, OutOfGasError, SuccessReason};

// pub struct Log(RevmLog);


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
                RevmExecutionResult::Success { reason, .. } => match reason {
                    SuccessReason::Stop => String::from("Stop"),
                    SuccessReason::Return => String::from("Return"),
                    SuccessReason::SelfDestruct => String::from("SelfDestruct"),
                }
                RevmExecutionResult::Revert { .. } => String::from("Revert"),
                RevmExecutionResult::Halt { reason, .. } => match reason {
                    HaltReason::OutOfGas(out_of_gas) => match out_of_gas {
                        OutOfGasError::Basic => String::from("OutOfGas:Basic"),
                        OutOfGasError::MemoryLimit => String::from("OutOfGas:MemoryLimit"),
                        OutOfGasError::Memory => String::from("OutOfGas:Memory"),
                        OutOfGasError::Precompile => String::from("OutOfGas:Precompile"),
                        OutOfGasError::InvalidOperand => String::from("OutOfGas:InvalidOperand"),
                    },
                    HaltReason::OpcodeNotFound => String::from("OpcodeNotFound"),
                    HaltReason::InvalidFEOpcode => String::from("InvalidFEOpcode"),
                    HaltReason::InvalidJump => String::from("InvalidJump"),
                    HaltReason::NotActivated => String::from("NotActivated"),
                    HaltReason::StackUnderflow => String::from("StackUnderflow"),
                    HaltReason::StackOverflow => String::from("StackOverflow"),
                    HaltReason::OutOfOffset => String::from("OutOfOffset"),
                    HaltReason::CreateCollision => String::from("CreateCollision"),
                    HaltReason::PrecompileError => String::from("PrecompileError"),
                    HaltReason::NonceOverflow => String::from("NonceOverflow"),
                    HaltReason::CreateContractSizeLimit => String::from("CreateContractSizeLimit"),
                    HaltReason::CreateContractStartingWithEF => String::from("CreateContractStartingWithEF"),
                    HaltReason::CreateInitCodeSizeLimit => String::from("CreateInitCodeSizeLimit"),
                    HaltReason::OverflowPayment => String::from("OverflowPayment"),
                    HaltReason::StateChangeDuringStaticCall => String::from("StateChangeDuringStaticCall"),
                    HaltReason::CallNotAllowedInsideStatic => String::from("CallNotAllowedInsideStatic"),
                    HaltReason::OutOfFunds => String::from("OutOfFunds"),
                    HaltReason::CallTooDeep => String::from("CallTooDeep"),
                    _ => String::from("Unknown"),
                }
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
