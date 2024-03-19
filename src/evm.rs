use std::collections::HashMap;
use std::fmt::Debug;
use std::io::stdout;
use pyo3::{PyErr, pymethods, PyResult, pyclass};
use pyo3::ffi::PySys_WriteStdout;

use revm::{Database, Evm, inspector_handle_register, primitives::U256};
use revm::DatabaseCommit;
use revm::precompile::{Address, Bytes};
use revm::precompile::B256;
use revm::primitives::{AccountInfo as RevmAccountInfo, BlockEnv, CreateScheme, Env as RevmEnv, EnvWithHandlerCfg, HandlerCfg, Output, ResultAndState, SpecId, State, TransactTo, TxEnv};
use revm::primitives::ExecutionResult::Success;
use revm::inspectors::TracerEip3155;
use tracing::{trace, warn};

use crate::{types::{AccountInfo, Env}, utils::{addr, pydict, pyerr}};
use crate::database::DB;
use crate::pystdout::PySysStdout;

// In Py03 we use vec<u8> to represent bytes
type PyBytes = Vec<u8>;

#[derive(Clone, Debug)]
#[pyclass]
pub struct EVM {
    /// The underlying `Database` that contains the EVM storage.
    pub db: DB,
    /// The EVM environment.
    pub env: RevmEnv,
    /// the current handler configuration
    pub handler_cfg: HandlerCfg,
    /// The gas limit for calls and deployments. This is different from the gas limit imposed by
    /// the passed in environment, as those limits are used by the EVM for certain opcodes like
    /// `gaslimit`.
    gas_limit: U256,

    /// whether to trace the execution to stdout
    tracing: bool,
}

#[pymethods]
impl EVM {
    /// Create a new EVM instance.
    #[new]
    #[pyo3(signature = (env=None, fork_url=None, fork_block_number=None, gas_limit=18446744073709551615, tracing=false, spec_id="SHANGHAI"))]
    fn new(
        env: Option<Env>,
        fork_url: Option<&str>,
        fork_block_number: Option<&str>,
        gas_limit: u64,
        tracing: bool,
        spec_id: &str,
    ) -> PyResult<Self> {
        Ok(EVM {
            db: fork_url.map(|url| DB::new_fork(url, fork_block_number)).unwrap_or(Ok(DB::new_memory()))?,
            env: env.unwrap_or_default().into(),
            gas_limit: U256::from(gas_limit),
            handler_cfg: HandlerCfg::new(SpecId::from(spec_id)),
            tracing,
        })
    }

    /// Get basic account information.
    fn basic(&mut self, address: &str) -> PyResult<AccountInfo> {
        Ok(self.db.basic(addr(address)?)?.unwrap_or_default().into())
    }

    /// Get account code by its hash.
    #[pyo3(signature = (code_hash))]
    fn code_by_hash(&mut self, code_hash: &str) -> PyResult<PyBytes> {
        let hash = code_hash.parse::<B256>().map_err(pyerr)?;
        Ok(self.db.code_by_hash(hash)?.bytecode.to_vec())
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: &str, index: U256) -> PyResult<U256> {
        Ok(self.db.storage(addr(address)?, index)?)
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: U256) -> PyResult<PyBytes> {
        Ok(self.db.block_hash(number)?.to_vec())
    }

    /// Inserts the provided account information in the database at the specified address.
    fn insert_account_info(
        &mut self,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        self.db.insert_account_info(addr(address)?, info.clone().into());
        Ok(())
    }

    /// Set the balance of a given address.
    fn set_balance(&mut self, address: &str, balance: U256) -> PyResult<()> {
        let mut info = self.db.basic(addr(address)?)?.unwrap_or_default();
        info.balance = balance;
        self.db.insert_account_info(addr(address)?, info);
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(&mut self, address: &str) -> PyResult<U256> {
        let RevmAccountInfo { balance, .. } = self.db.basic(addr(address)?)?.unwrap_or_default();
        Ok(balance)
    }

    /// runs a raw call and returns the result
    #[pyo3(signature = (caller, to, calldata=None, value=None))]
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
                self.db.commit(state);
                Ok(data.to_vec())
            },
            Err(e) => Err(e),
        }
    }

    #[pyo3(signature = (caller, to, calldata=None, value=None))]
    pub fn call_raw(
        &mut self,
        caller: &str,
        to: &str,
        calldata: Option<PyBytes>,
        value: Option<U256>,
    ) -> PyResult<(PyBytes, HashMap<String, AccountInfo>)> {
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
                self.db.commit(state);
                Ok(format!("{:?}", address))
            },
            Err(e) => Err(e),
        }
    }

    #[getter]
    fn env(&self) -> Env {
        self.env.clone().into()
    }

    #[getter]
    fn tracing(&self) -> bool {
        self.tracing
    }
}

impl EVM {
    /// Creates the environment to use when executing a transaction in a test context
    ///
    /// If using a backend with cheatcodes, `tx.gas_price` and `block.number` will be overwritten by
    /// the cheatcode state inbetween calls.
    fn build_test_env(
        &self,
        caller: Address,
        transact_to: TransactTo,
        data: Bytes,
        value: U256,
    ) -> EnvWithHandlerCfg {
        let env = revm::primitives::Env {
            cfg: self.env.cfg.clone(),
            // We always set the gas price to 0, so we can execute the transaction regardless of
            // network conditions - the actual gas price is kept in `evm.block` and is applied by
            // the cheatcode handler if it is enabled
            block: BlockEnv {
                basefee: U256::ZERO,
                gas_limit: self.gas_limit,
                ..self.env.block.clone()
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
                ..self.env.tx.clone()
            },
        };

        EnvWithHandlerCfg::new_with_spec_id(Box::new(env), self.handler_cfg.spec_id)
    }

    /// Deploys a contract using the given `env` and commits the new state to the underlying
    /// database
    fn deploy_with_env(&mut self, env: EnvWithHandlerCfg) -> PyResult<(Address, State)> {
        debug_assert!(
            matches!(env.tx.transact_to, TransactTo::Create(_)),
            "Expect create transaction"
        );
        trace!(sender=?env.tx.caller, "deploying contract");

        let ResultAndState { result, state } = self.run_env(env)?;

        match &result {
            Success { reason, gas_used, gas_refunded, logs, output } => {
                warn!(reason=?reason, gas_used, gas_refunded, "contract deployed");
                match output {
                    Output::Create(_, address) => {
                        Ok((address.unwrap(), state))
                    }
                    _ => Err(pyerr("Invalid output")),
                }
            },
            _ => Err(pyerr(result.clone())),
        }
    }

    fn call_raw_with_env(&mut self, env: EnvWithHandlerCfg) -> PyResult<(Bytes, State)>
    {
        debug_assert!(
            matches!(env.tx.transact_to, TransactTo::Call(_)),
            "Expect call transaction"
        );
        trace!(sender=?env.tx.caller, "deploying contract");

        let ResultAndState { result, state } = self.run_env(env)?;

        match &result {
            Success { reason, gas_used, gas_refunded, logs, output } => {
                let data = output.clone().into_data();
                trace!(reason=?reason, gas_used, gas_refunded, "call done");
                Ok((data, state))
            },
            // todo: state might have changed even if the call failed
            _ => Err(pyerr(result.clone())),
        }
    }

    fn run_env(&mut self, env: EnvWithHandlerCfg) -> Result<ResultAndState, PyErr>
    {
        let builder = Evm::builder()
            .with_db(&mut self.db);

        let result =
            if self.tracing {
                let tracer = TracerEip3155::new(Box::new(PySysStdout {}), true);
                builder
                    .with_external_context(tracer)
                    .with_env_with_handler_cfg(env)
                    .append_handler_register(inspector_handle_register)
                    .build()
                    .transact()
            } else {
                builder
                    .with_env_with_handler_cfg(env)
                    .build()
                    .transact()
            };
        Ok(result.map_err(pyerr)?)
    }
}
