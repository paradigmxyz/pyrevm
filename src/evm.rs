use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::mem::replace;

use pyo3::exceptions::{PyKeyError, PyOverflowError};
use pyo3::types::PyBytes;
use pyo3::{pyclass, pymethods, PyObject, PyResult, Python};
use revm::precompile::{Address, Bytes};
use revm::primitives::ExecutionResult::Success;
use revm::primitives::{
    BlockEnv as RevmBlockEnv, CreateScheme, Env as RevmEnv, ExecutionResult as RevmExecutionResult,
    HandlerCfg, Output, SpecId, TransactTo, TxEnv as RevmTxEnv,
};
use revm::{primitives::U256, Evm, EvmContext, JournalCheckpoint as RevmCheckpoint};
use tracing::trace;

use crate::database::DB;
use crate::executor::call_evm;
use crate::types::{PyByteVec, PyDB};
use crate::{
    types::{AccountInfo, BlockEnv, Env, ExecutionResult, JournalCheckpoint, TxEnv},
    utils::{addr, pyerr},
};

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
    #[pyo3(get, set)]
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
    #[pyo3(signature = (env = None, fork_url = None, fork_block = None, gas_limit = 18446744073709551615, tracing = false, spec_id = "LATEST"))]
    fn new(
        env: Option<Env>,
        fork_url: Option<&str>,
        fork_block: Option<&str>,
        gas_limit: u64,
        tracing: bool,
        spec_id: &str,
    ) -> PyResult<Self> {
        let spec = SpecId::from(spec_id);
        let env = env.unwrap_or_default().into();
        let db = fork_url
            .map(|url| DB::new_fork(url, fork_block))
            .unwrap_or(Ok(DB::new_memory()))?;

        let Evm { context, .. } = Evm::builder().with_env(Box::new(env)).with_db(db).build();
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
        self.checkpoints
            .insert(checkpoint, self.context.journaled_state.checkpoint());
        Ok(checkpoint)
    }

    fn revert(&mut self, checkpoint: JournalCheckpoint) -> PyResult<()> {
        if self.context.journaled_state.depth == 0 {
            return Err(PyOverflowError::new_err(format!(
                "No checkpoint to revert to: {:?}",
                self.context.journaled_state
            )));
        }

        if let Some(revm_checkpoint) = self.checkpoints.remove(&checkpoint) {
            self.context
                .journaled_state
                .checkpoint_revert(revm_checkpoint);
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

    fn get_code(&mut self, address: &str, py: Python<'_>) -> PyResult<Option<PyObject>> {
        let (code, _) = self.context.code(addr(address)?).map_err(pyerr)?;
        if code.is_empty() {
            return Ok(None);
        }
        Ok(Some(PyBytes::new(py, code.bytecode.as_ref()).into()))
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: &str, index: U256) -> PyResult<U256> {
        let address = addr(address)?;
        // `sload` expects the account to be already loaded.
        let _ = self.context.load_account(address).map_err(pyerr)?;
        let (value, _) = self.context.sload(address, index).map_err(pyerr)?;
        Ok(value)
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: U256, py: Python<'_>) -> PyResult<PyObject> {
        let hash = self.context.block_hash(number).map_err(pyerr)?;
        Ok(PyBytes::new(py, hash.as_ref()).into())
    }

    /// Inserts the provided account information in the database at the specified address.
    fn insert_account_info(&mut self, address: &str, info: AccountInfo) -> PyResult<()> {
        let target = addr(address)?;
        match self.context.journaled_state.state.get_mut(&target) {
            // account is cold, just insert into the DB, so it's retrieved next time
            None => self.context.db.insert_account_info(target, info.into()),
            // just replace the account info
            Some(acc) => acc.info = info.into(),
        }
        Ok(())
    }

    /// Set the balance of a given address.
    fn set_balance(&mut self, address: &str, balance: U256) -> PyResult<()> {
        let address = addr(address)?;
        let (account, _) = self.context.load_account(address).map_err(pyerr)?;
        account.info.balance = balance;
        self.context.journaled_state.touch(&address);
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(&mut self, address: &str) -> PyResult<U256> {
        let (balance, _) = self.context.balance(addr(address)?).map_err(pyerr)?;
        Ok(balance)
    }

    #[pyo3(signature = (caller, to, calldata = None, value = None, gas = None, gas_price = None, is_static = false))]
    pub fn message_call(
        &mut self,
        caller: &str,
        to: &str,
        calldata: Option<PyByteVec>,
        value: Option<U256>,
        gas: Option<U256>,
        gas_price: Option<U256>,
        is_static: bool,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let env = self.build_test_env(
            addr(caller)?,
            TransactTo::Call(addr(to)?),
            calldata.unwrap_or_default().into(),
            value.unwrap_or_default(),
            gas,
            gas_price,
        );
        match self.call_with_env(env, is_static) {
            Ok(data) => Ok(PyBytes::new(py, data.as_ref()).into()),
            Err(e) => Err(e),
        }
    }

    /// Deploy a contract with the given code.
    #[pyo3(signature = (deployer, code, value = None, gas = None, gas_price = None, is_static = false, _abi = None))]
    fn deploy(
        &mut self,
        deployer: &str,
        code: PyByteVec,
        value: Option<U256>,
        gas: Option<U256>,
        gas_price: Option<U256>,
        is_static: bool,
        _abi: Option<&str>,
    ) -> PyResult<String> {
        let env = self.build_test_env(
            addr(deployer)?,
            TransactTo::Create(CreateScheme::Create),
            code.into(),
            value.unwrap_or_default(),
            gas,
            gas_price,
        );
        match self.deploy_with_env(env, is_static) {
            Ok((_, address)) => Ok(format!("{:?}", address)),
            Err(e) => Err(e),
        }
    }

    #[getter]
    fn env(&self) -> Env {
        (*self.context.env).clone().into()
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

    #[getter]
    fn journal_str(&self) -> String {
        format!("{:?}", self.context.journaled_state)
    }

    #[getter]
    fn db_accounts(&self) -> PyDB {
        self.context
            .db
            .get_accounts()
            .iter()
            .map(|(address, db_acc)| (address.to_string(), db_acc.info.clone().into()))
            .collect()
    }

    #[getter]
    fn journal_state(&self) -> PyDB {
        self.context
            .journaled_state
            .state
            .iter()
            .map(|(address, acc)| (address.to_string(), acc.info.clone().into()))
            .collect()
    }

    fn set_block_env(&mut self, block: BlockEnv) {
        self.context.env.block = block.into();
    }

    fn set_tx_env(&mut self, tx: TxEnv) {
        self.context.env.tx = tx.into();
    }

    fn reset_transient_storage(&mut self) {
        self.context.journaled_state.transient_storage.clear();
    }

    fn __str__(&self) -> String {
        format!("{:?}", self)
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
        gas: Option<U256>,
        gas_price: Option<U256>,
    ) -> RevmEnv {
        RevmEnv {
            cfg: self.context.env.cfg.clone(),
            // We always set the gas price to 0, so we can execute the transaction regardless of
            // network conditions - the actual gas price is kept in `evm.block` and is applied by
            // the cheatcode handler if it is enabled
            block: RevmBlockEnv {
                basefee: U256::ZERO,
                gas_limit: self.gas_limit,
                ..self.context.env.block.clone()
            },
            tx: RevmTxEnv {
                caller,
                transact_to,
                data,
                value,
                // As above, we set the gas price to 0.
                gas_price: gas_price.unwrap_or(U256::ZERO),
                gas_priority_fee: None,
                gas_limit: gas.unwrap_or(self.gas_limit).to(),
                ..self.context.env.tx.clone()
            },
        }
    }

    /// Deploys a contract using the given `env` and commits the new state to the underlying
    /// database
    fn deploy_with_env(&mut self, env: RevmEnv, is_static: bool) -> PyResult<(Bytes, Address)> {
        debug_assert!(
            matches!(env.tx.transact_to, TransactTo::Create(_)),
            "Expect create transaction"
        );
        trace!(sender=?env.tx.caller, "deploying contract");

        let result = self.run_env(env, is_static)?;

        if let Success { output, .. } = result {
            if let Output::Create(out, address) = output {
                Ok((out, address.unwrap()))
            } else {
                Err(pyerr(output))
            }
        } else {
            Err(pyerr(result))
        }
    }

    fn call_with_env(&mut self, env: RevmEnv, is_static: bool) -> PyResult<Bytes> {
        debug_assert!(
            matches!(env.tx.transact_to, TransactTo::Call(_)),
            "Expect call transaction"
        );
        trace!(sender=?env.tx.caller, "deploying contract");

        let result = self.run_env(env, is_static)?;
        if let Success { output, .. } = result {
            if let Output::Call(_) = output {
                Ok(output.into_data())
            } else {
                Err(pyerr(output))
            }
        } else {
            Err(pyerr(result))
        }
    }

    fn run_env(&mut self, env: RevmEnv, is_static: bool) -> PyResult<RevmExecutionResult> {
        self.context.env = Box::new(env);
        let evm_context: EvmContext<DB> =
            replace(&mut self.context, EvmContext::new(DB::new_memory()));
        let (result, evm_context) =
            call_evm(evm_context, self.handler_cfg, self.tracing, is_static);
        self.context = evm_context;
        self.result = result.as_ref().ok().cloned();
        result
    }
}
