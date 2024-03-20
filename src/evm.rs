use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::mem::replace;

use pyo3::{pyclass, PyErr, pymethods, PyResult};
use pyo3::exceptions::{PyKeyError};
use revm::{Context, ContextWithHandlerCfg, Database, Evm, EvmContext, inspector_handle_register, primitives::U256};
use revm::DatabaseCommit;
use revm::inspectors::TracerEip3155;
use revm::precompile::{Address, Bytes};
use revm::primitives::{BlockEnv, CreateScheme, Env as RevmEnv, ExecutionResult as RevmExecutionResult, HandlerCfg, Output, ResultAndState, SpecId, State, TransactTo, TxEnv};
use revm::primitives::ExecutionResult::Success;
use tracing::trace;

use crate::{types::{AccountInfo, Env, ExecutionResult}, utils::{addr, pydict, pyerr}};
use crate::database::DB;
use crate::pystdout::PySysStdout;
use crate::types::{PyBytes, PyDB, Checkpoint};
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
    /// We cannot use Revm's checkpointing mechanism as it is limited to transaction scope
    checkpoints: HashMap<i32, DB>,
    checkpoint: Checkpoint,

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
            checkpoint: 0,
            result: None,
        })
    }

    fn snapshot(&mut self) -> PyResult<Checkpoint> {
        self.checkpoint += 1;
        self.checkpoints.insert(self.checkpoint, self.context.db.clone());
        Ok(self.checkpoint)
    }

    fn revert(&mut self, checkpoint: Checkpoint) -> PyResult<()> {
        if let Some(db) = self.checkpoints.remove(&checkpoint) {
            self.context.db = db;
            Ok(())
        } else {
            Err(PyKeyError::new_err(format!("Invalid checkpoint {0}", checkpoint)))
        }
    }

    fn commit(&mut self) {
        self.checkpoints.clear();
    }

    /// Get basic account information.
    fn basic(&mut self, address: &str) -> PyResult<AccountInfo> {
        let info = self.context.db.basic(addr(address)?).map_err(pyerr)?.unwrap_or_default();
        Ok(info.clone().into())
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
        let target = addr(address)?;
        let mut info = self.context.db.basic(target).map_err(pyerr)?.unwrap_or_default().clone();
        info.balance = balance;
        self.context.db.insert_account_info(target, info);
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(&mut self, address: &str) -> PyResult<U256> {
        let info = self.context.db.basic(addr(address)?).map_err(pyerr)?.unwrap_or_default();
        Ok(info.balance)
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
    fn checkpoint_ids(&self) -> HashSet<Checkpoint> {
        self.checkpoints.keys().cloned().collect()
    }

    fn get_checkpoints(&self) -> HashMap<Checkpoint, PyDB> {
        self.checkpoints.iter().map(|(k, db)| (*k, to_hashmap(db.get_accounts()))).collect()
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

            (evm.transact().map_err(pyerr)?, evm.context.evm)
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

            (evm.transact().map_err(pyerr)?, evm.context.evm)
        };
        self.context = evm_context;
        self.result = Some(result_and_state.result.clone());
        Ok(result_and_state)
    }
}
