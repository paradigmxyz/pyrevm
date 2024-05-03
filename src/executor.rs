use std::mem::replace;

use pyo3::exceptions::PyRuntimeError;
use pyo3::PyResult;
use revm::inspectors::TracerEip3155;
use revm::precompile::Log;
use revm::primitives::TransactTo;
use revm::primitives::{ExecutionResult, ShanghaiSpec};
use revm::{
    inspector_handle_register, Context, ContextWithHandlerCfg, Evm, EvmContext, FrameOrResult,
    FrameResult,
};
use revm_interpreter::primitives::HandlerCfg;
use revm_interpreter::{gas, CallInputs, CreateInputs, SuccessOrHalt};

use crate::database::DB;
use crate::utils::pyerr;

/// Calls the EVM with the given context and handler configuration.
pub(crate) fn call_evm(
    evm_context: EvmContext<DB>,
    handler_cfg: HandlerCfg,
    tracing: bool,
    is_static: bool,
) -> (PyResult<ExecutionResult>, EvmContext<DB>) {
    if tracing {
        let tracer = TracerEip3155::new(Box::new(crate::pystdout::PySysStdout {}));
        let mut evm = Evm::builder()
            .with_context_with_handler_cfg(ContextWithHandlerCfg {
                cfg: handler_cfg,
                context: Context {
                    evm: evm_context,
                    external: tracer,
                },
            })
            .append_handler_register(inspector_handle_register)
            .build();
        (run_evm(&mut evm, is_static), evm.context.evm)
    } else {
        let mut evm = Evm::builder()
            .with_context_with_handler_cfg(ContextWithHandlerCfg {
                cfg: handler_cfg,
                context: Context {
                    evm: evm_context,
                    external: (),
                },
            })
            .build();
        (run_evm(&mut evm, is_static), evm.context.evm)
    }
}

/// Calls the given evm. This is originally a copy of revm::Evm::transact, but it calls our own output function
fn run_evm<EXT>(evm: &mut Evm<'_, EXT, DB>, is_static: bool) -> PyResult<ExecutionResult> {
    let logs_i = evm.context.evm.journaled_state.logs.len();

    evm.handler
        .validation()
        .env(&evm.context.evm.env)
        .map_err(pyerr)?;
    let initial_gas_spend = evm
        .handler
        .validation()
        .initial_tx_gas(&evm.context.evm.env)
        .map_err(|e| {
            let tx = &evm.context.evm.env.tx;
            PyRuntimeError::new_err(format!(
                "Initial gas spend is {} but gas limit is {}. Error: {:?}",
                gas::validate_initial_tx_gas::<ShanghaiSpec>(
                    &tx.data,
                    tx.transact_to.is_create(),
                    &tx.access_list
                ),
                tx.gas_limit,
                e
            ))
        })?;

    evm.handler
        .validation()
        .tx_against_state(&mut evm.context)
        .map_err(pyerr)?;

    let ctx = &mut evm.context;
    let pre_exec = evm.handler.pre_execution();

    // load access list and beneficiary if needed.
    pre_exec.load_accounts(ctx).map_err(pyerr)?;

    // load precompiles
    ctx.evm.set_precompiles(pre_exec.load_precompiles());

    // deduce caller balance with its limit.
    pre_exec.deduct_caller(ctx).map_err(pyerr)?;

    let gas_limit = ctx.evm.env.tx.gas_limit - initial_gas_spend;

    let exec = evm.handler.execution();
    // call inner handling of call/create
    let first_frame_or_result = match ctx.evm.env.tx.transact_to {
        TransactTo::Call(_) => exec
            .call(ctx, call_inputs(&ctx, gas_limit, is_static))
            .map_err(pyerr)?,
        TransactTo::Create(_) => exec
            .create(
                ctx,
                CreateInputs::new_boxed(&ctx.evm.env.tx, gas_limit).unwrap(),
            )
            .map_err(pyerr)?,
    };

    // Starts the main running loop.
    let mut result = match first_frame_or_result {
        FrameOrResult::Frame(first_frame) => evm.start_the_loop(first_frame).map_err(pyerr)?,
        FrameOrResult::Result(result) => result,
    };

    let ctx = &mut evm.context;

    // handle output of call/create calls.
    evm.handler
        .execution()
        .last_frame_return(ctx, &mut result)
        .map_err(pyerr)?;

    let post_exec = evm.handler.post_execution();
    // Reimburse the caller
    post_exec
        .reimburse_caller(ctx, result.gas())
        .map_err(pyerr)?;
    // Reward beneficiary
    post_exec
        .reward_beneficiary(ctx, result.gas())
        .map_err(pyerr)?;

    let logs = ctx.evm.journaled_state.logs[logs_i..].to_vec();

    // Returns output of transaction.
    output(ctx, result, logs)
}

fn call_inputs<EXT>(
    ctx: &&mut Context<EXT, DB>,
    gas_limit: u64,
    is_static: bool,
) -> Box<CallInputs> {
    let mut inputs = CallInputs::new_boxed(&ctx.evm.env.tx, gas_limit).unwrap();
    inputs.is_static = is_static;
    inputs
}

/// Returns the output of the transaction.
/// This is mostly copied from revm::handler::mainnet::post_execution::output
/// However, we removed the journal finalization to keep the transaction open.
#[inline]
fn output<EXT>(
    context: &mut Context<EXT, DB>,
    result: FrameResult,
    logs: Vec<Log>,
) -> PyResult<ExecutionResult> {
    replace(&mut context.evm.error, Ok(())).map_err(pyerr)?;
    // used gas with refund calculated.
    let gas_refunded = result.gas().refunded() as u64;
    let final_gas_used = result.gas().spent() - gas_refunded;
    let output = result.output();
    let instruction_result = result.into_interpreter_result();

    let result = match instruction_result.result.into() {
        SuccessOrHalt::Success(reason) => ExecutionResult::Success {
            reason,
            gas_used: final_gas_used,
            gas_refunded,
            logs,
            output,
        },
        SuccessOrHalt::Revert => ExecutionResult::Revert {
            gas_used: final_gas_used,
            output: output.into_data(),
        },
        SuccessOrHalt::Halt(reason) => ExecutionResult::Halt {
            reason,
            gas_used: final_gas_used,
        },
        // Only two internal return flags.
        SuccessOrHalt::FatalExternalError
        | SuccessOrHalt::InternalContinue
        | SuccessOrHalt::InternalCallOrCreate => {
            panic!("Internal return flags should remain internal {instruction_result:?}")
        }
    };

    Ok(result)
}
