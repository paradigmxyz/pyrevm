use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::mem::replace;

use pyo3::{pyclass, PyErr, pymethods, PyResult};
use pyo3::exceptions::{PyKeyError, PyOverflowError};
use revm::{Context, ContextWithHandlerCfg, Database, Evm, EvmContext, FrameOrResult, FrameResult, inspector_handle_register, JournalCheckpoint as RevmCheckpoint, primitives::U256};
use revm::DatabaseCommit;
use revm::inspectors::TracerEip3155;
use revm::precompile::{Address, Bytes};
use revm::primitives::{BlockEnv, CreateScheme, Env as RevmEnv, ExecutionResult as RevmExecutionResult, HandlerCfg, Output, ResultAndState, SpecId, State, TransactTo, TxEnv};
use revm::primitives::ExecutionResult::Success;
use revm_interpreter::{CallInputs, CreateInputs, SuccessOrHalt};
use tracing::trace;

use crate::{types::{AccountInfo, Env, ExecutionResult, JournalCheckpoint}, utils::{addr, pydict, pyerr}};
use crate::database::DB;
use crate::pystdout::PySysStdout;
use crate::types::{PyBytes, PyDB};
use crate::utils::to_hashmap;

#[derive(Debug)]
#[pyclass]
pub struct EVM {
    /// Context of execution.
    context: EvmContext<DB>,

    /// Handler configuration.
    handler_cfg: HandlerCfg,

    /// The gas limit for calls and deployments. This is different from the gas limit imposed by
    /// the passed in environment, as those limits are used by the EVM for certain opcodes like
    /// `gaslimit`.
    gas_limit: U256,

    /// whether to trace the execution to stdout
    tracing: bool,

    /// Checkpoints for reverting state
    /// We cannot use Revm's checkpointing mechanism as it is not serializable
    checkpoints: HashMap<JournalCheckpoint, RevmCheckpoint>,

    /// The result of the last transaction
    result: Option<RevmExecutionResult>,
}

#[pymethods]
impl EVM {
    /// Create a new EVM instance.
    #[new]
    #[pyo3(signature = (env = None, fork_url = None, fork_block_number = None, gas_limit = 18446744073709551615, tracing = false, spec_id = "LATEST"))]
    fn new(
        env: Option<Env>,
        fork_url: Option<&str>,
        fork_block_number: Option<&str>,
        gas_limit: u64,
        tracing: bool,
        spec_id: &str,
    ) -> PyResult<Self> {
        let spec = SpecId::from(spec_id);
        let env = env.unwrap_or_default().into();
        let db = fork_url.map(|url| DB::new_fork(url, fork_block_number)).unwrap_or(Ok(DB::new_memory()))?;

        let Evm { context, .. } = Evm::builder()
            .with_env(Box::new(env))
            .with_db(db)
            .build();
        Ok(EVM {
            context: context.evm,
            gas_limit: U256::from(gas_limit),
            handler_cfg: HandlerCfg::new(spec),
            tracing,
            checkpoints: HashMap::new(),
            result: None,
        })
    }

    fn snapshot(&mut self) -> PyResult<JournalCheckpoint> {
        let checkpoint = JournalCheckpoint {
            log_i: self.context.journaled_state.logs.len(),
            journal_i: self.context.journaled_state.journal.len(),
        };
        self.checkpoints.insert(checkpoint, self.context.journaled_state.checkpoint());
        Ok(checkpoint)
    }

    fn revert(&mut self, checkpoint: JournalCheckpoint) -> PyResult<()> {
        if self.context.journaled_state.depth == 0 {
            return Err(PyOverflowError::new_err(format!("No checkpoint to revert to: {:?}", self.context.journaled_state)));
        }

        if let Some(revm_checkpoint) = self.checkpoints.remove(&checkpoint) {
            self.context.journaled_state.checkpoint_revert(revm_checkpoint);
            Ok(())
        } else {
            Err(PyKeyError::new_err("Invalid checkpoint"))
        }
    }

    fn commit(&mut self) {
        self.context.journaled_state.checkpoint_commit();
    }

    /// Get basic account information.
    fn basic(&mut self, address: &str) -> PyResult<AccountInfo> {
        let (account, _) = self.context.load_account(addr(address)?).map_err(pyerr)?;
        Ok(account.info.clone().into())
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: &str, index: U256) -> PyResult<U256> {
        Ok(self.context.db.storage(addr(address)?, index)?)
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: U256) -> PyResult<PyBytes> {
        Ok(self.context.block_hash(number).map_err(pyerr)?.to_vec())
    }

    /// Inserts the provided account information in the database at the specified address.
    fn insert_account_info(
        &mut self,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        self.context.db.insert_account_info(addr(address)?, info.clone().into());
        Ok(())
    }

    /// Set the balance of a given address.
    fn set_balance(&mut self, address: &str, balance: U256) -> PyResult<()> {
        let address_ = addr(address)?;
        let account = {
            let (account, _) = self.context.load_account(address_).map_err(pyerr)?;
            account.info.balance = balance;
            account.clone()
        };
        self.context.db.insert_account_info(address_, account.info.clone());
        self.context.journaled_state.state.insert(address_, account);
        self.context.journaled_state.touch(&address_);
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(&mut self, address: &str) -> PyResult<U256> {
        let (balance, _) = self.context.balance(addr(address)?).map_err(pyerr)?;
        Ok(balance)
    }

    /// runs a raw call and returns the result
    #[pyo3(signature = (caller, to, calldata = None, value = None))]
    pub fn call_raw_committing(
        &mut self,
        caller: &str,
        to: &str,
        calldata: Option<PyBytes>,
        value: Option<U256>,
    ) -> PyResult<PyBytes> {
        let env = self.build_test_env(addr(caller)?, TransactTo::Call(addr(to)?), calldata.unwrap_or_default().into(), value.unwrap_or_default().into());
        match self.call_raw_with_env(env)
        {
            Ok((data, state)) => {
                self.context.db.commit(state);
                Ok(data.to_vec())
            }
            Err(e) => Err(e),
        }
    }

    #[pyo3(signature = (caller, to, calldata = None, value = None))]
    pub fn call_raw(
        &mut self,
        caller: &str,
        to: &str,
        calldata: Option<PyBytes>,
        value: Option<U256>,
    ) -> PyResult<(PyBytes, PyDB)> {
        let env = self.build_test_env(addr(caller)?, TransactTo::Call(addr(to)?), calldata.unwrap_or_default().into(), value.unwrap_or_default().into());
        match self.call_raw_with_env(env)
        {
            Ok((data, state)) => Ok((data.to_vec(), pydict(state))),
            Err(e) => Err(e),
        }
    }

    /// Deploy a contract with the given code.
    fn deploy(
        &mut self,
        deployer: &str,
        code: Option<PyBytes>,
        value: Option<U256>,
        _abi: Option<&str>,
    ) -> PyResult<String> {
        let env = self.build_test_env(addr(deployer)?, TransactTo::Create(CreateScheme::Create), code.unwrap_or_default().into(), value.unwrap_or_default());
        match self.deploy_with_env(env)
        {
            Ok((address, state)) => {
                self.context.db.commit(state);
                Ok(format!("{:?}", address))
            }
            Err(e) => Err(e),
        }
    }

    #[getter]
    fn env(&self) -> Env {
        (*self.context.env).clone().into()
    }

    #[getter]
    fn tracing(&self) -> bool {
        self.tracing
    }

    #[getter]
    fn result(&self) -> Option<ExecutionResult> {
        self.result.clone().map(|r| r.into())
    }

    #[getter]
    fn checkpoint_ids(&self) -> HashSet<JournalCheckpoint> {
        self.checkpoints.keys().cloned().collect()
    }

    #[getter]
    fn journal_depth(&self) -> usize {
        self.context.journaled_state.depth
    }
    #[getter]
    fn journal_len(&self) -> usize {
        self.context.journaled_state.journal.len()
    }

    fn get_accounts(&self) -> PyDB {
        to_hashmap(self.context.db.get_accounts())
    }

}

impl EVM {
    /// Creates the environment to use when executing a transaction in a test context
    ///
    /// If using a backend with cheat codes, `tx.gas_price` and `block.number` will be overwritten by
    /// the cheatcode state inbetween calls.
    fn build_test_env(
        &self,
        caller: Address,
        transact_to: TransactTo,
        data: Bytes,
        value: U256,
    ) -> RevmEnv {
        RevmEnv {
            cfg: self.context.env.cfg.clone(),
            // We always set the gas price to 0, so we can execute the transaction regardless of
            // network conditions - the actual gas price is kept in `evm.block` and is applied by
            // the cheatcode handler if it is enabled
            block: BlockEnv {
                basefee: U256::ZERO,
                gas_limit: self.gas_limit,
                ..self.context.env.block.clone()
            },
            tx: TxEnv {
                caller,
                transact_to,
                data,
                value,
                // As above, we set the gas price to 0.
                gas_price: U256::ZERO,
                gas_priority_fee: None,
                gas_limit: self.gas_limit.to(),
                ..self.context.env.tx.clone()
            },
        }
    }

    /// Deploys a contract using the given `env` and commits the new state to the underlying
    /// database
    fn deploy_with_env(&mut self, env: RevmEnv) -> PyResult<(Address, State)> {
        debug_assert!(
            matches!(env.tx.transact_to, TransactTo::Create(_)),
            "Expect create transaction"
        );
        trace!(sender=?env.tx.caller, "deploying contract");

        let ResultAndState { result, state } = self.run_env(env)?;

        match &result {
            Success { output, .. } => {
                match output {
                    Output::Create(_, address) => {
                        Ok((address.unwrap(), state))
                    }
                    _ => Err(pyerr("Invalid output")),
                }
            }
            _ => Err(pyerr(result.clone())),
        }
    }

    fn call_raw_with_env(&mut self, env: RevmEnv) -> PyResult<(Bytes, State)> {
        debug_assert!(
            matches!(env.tx.transact_to, TransactTo::Call(_)),
            "Expect call transaction"
        );
        trace!(sender=?env.tx.caller, "deploying contract");

        let ResultAndState { result, state } = self.run_env(env)?;

        match &result {
            Success { output, .. } => {
                let data = output.clone().into_data();
                Ok((data, state))
            }
            // todo: state might have changed even if the call failed
            _ => Err(pyerr(result.clone())),
        }
    }

    fn run_env(&mut self, env: RevmEnv) -> Result<ResultAndState, PyErr>
    {
        self.context.env = Box::new(env);

        // temporarily take the context out of the EVM instance
        let evm_context: EvmContext<DB> = replace(&mut self.context, EvmContext::new(DB::new_memory()));

        let (result_and_state, evm_context) = if self.tracing {
            let tracer = TracerEip3155::new(Box::new(PySysStdout {}), true);
            let mut evm = Evm::builder()
                .with_context_with_handler_cfg(ContextWithHandlerCfg {
                    cfg: self.handler_cfg,
                    context: Context {
                        evm: evm_context,
                        external: tracer,
                    },
                })
                .append_handler_register(inspector_handle_register)
                .build();

            (Self::call(&mut evm)?, evm.context.evm)
        } else {
            let mut evm = Evm::builder()
                .with_context_with_handler_cfg(ContextWithHandlerCfg {
                    cfg: self.handler_cfg,
                    context: Context {
                        evm: evm_context,
                        external: (),
                    },
                })
                .build();

            (Self::call(&mut evm)?, evm.context.evm)
        };
        self.context = evm_context;
        self.result = Some(result_and_state.result.clone());
        Ok(result_and_state)
    }
    
    fn call<EXT>(evm: &mut Evm<'_, EXT, DB>) -> PyResult<ResultAndState> {
        evm.handler.validation().env(&evm.context.evm.env).map_err(pyerr)?;
        let initial_gas_spend = evm
            .handler
            .validation()
            .initial_tx_gas(&evm.context.evm.env)
            .map_err(pyerr)?;
        evm.handler
            .validation()
            .tx_against_state(&mut evm.context)
            .map_err(pyerr)?;

        let output = Self::transact_preverified_inner(evm, initial_gas_spend)?;
        Ok(evm.handler.post_execution().end(&mut evm.context, Ok(output)).map_err(pyerr)?)
    }

    fn transact_preverified_inner<EXT>(evm: &mut Evm<'_, EXT, DB>, initial_gas_spend: u64) -> PyResult<ResultAndState> {
        let ctx = &mut evm.context;
        let pre_exec = evm.handler.pre_execution();

        // load access list and beneficiary if needed.
        pre_exec.load_accounts(ctx).map_err(pyerr)?;

        // load precompiles
        let precompiles = pre_exec.load_precompiles();
        ctx.evm.set_precompiles(precompiles);

        // deduce caller balance with its limit.
        pre_exec.deduct_caller(ctx).map_err(pyerr)?;

        let gas_limit = ctx.evm.env.tx.gas_limit - initial_gas_spend;

        let exec = evm.handler.execution();
        // call inner handling of call/create
        let first_frame_or_result = match ctx.evm.env.tx.transact_to {
            TransactTo::Call(_) => exec.call(
                ctx,
                CallInputs::new_boxed(&ctx.evm.env.tx, gas_limit).unwrap(),
            ).map_err(pyerr)?,
            TransactTo::Create(_) => exec.create(
                ctx,
                CreateInputs::new_boxed(&ctx.evm.env.tx, gas_limit).unwrap(),
            ).map_err(pyerr)?,
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
        post_exec.reimburse_caller(ctx, result.gas()).map_err(pyerr)?;
        // Reward beneficiary
        post_exec.reward_beneficiary(ctx, result.gas()).map_err(pyerr)?;
        // Returns output of transaction.
        Self::output(ctx, result)
    }

    /// Main return handle, returns the output of the transaction.
    #[inline]
    fn output<EXT>(
        context: &mut Context<EXT, DB>,
        result: FrameResult,
    ) -> PyResult<ResultAndState> {
        replace(&mut context.evm.error, Ok(())).map_err(pyerr)?;
        // used gas with refund calculated.
        let gas_refunded = result.gas().refunded() as u64;
        let final_gas_used = result.gas().spend() - gas_refunded;
        let output = result.output();
        let instruction_result = result.into_interpreter_result();

        // reset journal and return present state.
        let (state, logs) = context.evm.journaled_state.finalize();

        let result = match instruction_result.result.into() {
            SuccessOrHalt::Success(reason) => Success {
                reason,
                gas_used: final_gas_used,
                gas_refunded,
                logs,
                output,
            },
            SuccessOrHalt::Revert => revm::primitives::ExecutionResult::Revert {
                gas_used: final_gas_used,
                output: output.into_data(),
            },
            SuccessOrHalt::Halt(reason) => revm::primitives::ExecutionResult::Halt {
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

        Ok(ResultAndState { result, state })
    }
}
