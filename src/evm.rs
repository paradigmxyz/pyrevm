use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use ethers_core::types::BlockId;
use std::str::FromStr;

use pyo3::prelude::*;
use revm::{Database, DatabaseCommit, DatabaseRef, Evm, primitives::U256};
use revm::db::{CacheDB, EthersDB};
use revm::precompile::{Address, Bytes};
use revm::precompile::B256;
use revm::primitives::{AccountInfo as RevmAccountInfo, BlockEnv, CreateScheme, Env as RevmEnv, EnvWithHandlerCfg, ExecutionResult, HandlerCfg, Output, ResultAndState, SpecId, State, TransactTo, TxEnv};
use revm::primitives::ExecutionResult::Success;
use tracing::{trace, warn};
use ethers_providers::{Http, Provider};

use crate::{types::{AccountInfo, Env}, utils, utils::addr};
use crate::empty_db_wrapper::EmptyDBWrapper;
use crate::utils::pydict;

type MemDB = CacheDB<EmptyDBWrapper>;
type ForkDB = CacheDB<EthersDB<Provider<Http>>>;
#[derive(Clone, Debug)]
pub enum DB<'a> {
    Memory(&'a MemDB),
    Fork(&'a ForkDB),
}


#[derive(Clone, Debug)]
#[pyclass]
pub struct EVM {
    /// The underlying `Database` that contains the EVM storage.
    pub db: MemDB,
    /// The EVM environment.
    pub env: RevmEnv,
    /// the current handler configuration
    pub handler_cfg: HandlerCfg,
    /// The gas limit for calls and deployments. This is different from the gas limit imposed by
    /// the passed in environment, as those limits are used by the EVM for certain opcodes like
    /// `gaslimit`.
    gas_limit: U256,

    /// The fork to use for the EVM. If `None`, the EVM will use the memory database.
    fork: Option<ForkDB>,
}

impl EVM {
    pub fn db(&self) -> DB<'_> {
        if let Some(fork) = &self.fork {
            DB::Fork(&fork)
        } else {
            DB::Memory(&self.db)
        }
    }
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
        let fork_option: Option<PyResult<ForkDB>> = fork_url.map(
            |url| {
                let p = Provider::<Http>::try_from(url).map_err(utils::pyerr)?;
                let client = Arc::new(p);
                let block = fork_block_number.map(|n| BlockId::from_str(n)).map_or(Ok(None), |v| v.map(Some)).map_err(utils::pyerr)?;
                let db = EthersDB::new(client, block).unwrap();
                Ok(CacheDB::new(db))
            }
        );

        Ok(EVM {
            fork: fork_option.map_or(Ok(None), |v| v.map(Some)).map_err(utils::pyerr)?,
            db: MemDB::default(),
            env: env.unwrap_or_default().into(),
            gas_limit: U256::from(gas_limit),
            handler_cfg: HandlerCfg::new(SpecId::from(spec_id)),
        })
    }

    /// Get basic account information.
    fn basic(&mut self, address: &str) -> PyResult<AccountInfo> {
        let db_account = self.db.basic_ref(addr(address)?).map_err(utils::pyerr)?;
        Ok(db_account.unwrap_or_default().into())
    }

    /// Get account code by its hash.
    #[pyo3(signature = (code_hash))]
    fn code_by_hash(&mut self, code_hash: &str) -> PyResult<Vec<u8>> {
        let hash = code_hash.parse::<B256>().map_err(utils::pyerr)?;
        Ok(self.db.code_by_hash(hash).map(|c| c.bytecode.to_vec()).map_err(utils::pyerr)?)
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: &str, index: U256) -> PyResult<U256> {
        Ok(self.db.storage(addr(address)?, index).map_err(utils::pyerr)?)
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: U256) -> PyResult<Vec<u8>> {
        Ok(self.db.block_hash(number).map(|h| h.to_vec()).map_err(utils::pyerr)?)
    }

    /// Inserts the provided account information in the database at the specified address.
    fn insert_account_info(
        &mut self,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        let info = RevmAccountInfo::from(info);
        self.db.insert_account_info(addr(address)?, info.clone());
        assert_eq!(self.db.basic(addr(address)?).unwrap().unwrap().balance, info.balance);
        Ok(())
    }

    /// Set the balance of a given address.
    fn set_balance(&mut self, address: &str, balance: U256) -> PyResult<()> {
        let target = addr(address)?;
        let mut info = self.db.basic(target).map_err(utils::pyerr)?.unwrap_or_default();
        info.balance = balance;
        self.db.insert_account_info(target, info.clone());
        assert_eq!(self.db.load_account(target).map(|a| a.info.clone()).map_err(utils::pyerr)?, info);
        // assert_eq!(self.db.basic(target).map(|a| a.unwrap_or_default().balance).map_err(pyerr)?, balance);
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(&mut self, address: &str) -> PyResult<U256> {
        // Ok(self.db.basic(addr(address).map_err(pyerr)?)?.map(|acc| acc.balance))
        let acc = self.db.load_account(addr(address)?).map_err(utils::pyerr)?;
        // let db_account = self.db.basic(addr(address)?).map_err(pyerr)?.unwrap_or_default();
        // assert_eq!(db_account.balance, acc.info.balance);
        Ok(acc.info.balance)
    }

    /// runs a raw call and returns the result
    #[pyo3(signature = (caller, to, calldata=None, value=None))]
    pub fn call_raw_committing(
        &mut self,
        caller: &str,
        to: &str,
        calldata: Option<Vec<u8>>,
        value: Option<U256>,
    ) -> PyResult<Vec<u8>> {
        match call_raw(self, caller, to, calldata, value)
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
        &self,
        caller: &str,
        to: &str,
        calldata: Option<Vec<u8>>,
        value: Option<U256>,
    ) -> PyResult<(Vec<u8>, HashMap<String, AccountInfo>)> {
        match call_raw(self, caller, to, calldata, value)
        {
            // todo: return state to the caller
            Ok((data, state)) => Ok((data.to_vec(), pydict(state))),
            Err(e) => Err(e),
        }
    }

    /// Deploy a contract with the given code.
    fn deploy(
        &mut self,
        deployer: &str,
        code: Option<Vec<u8>>,
        value: Option<U256>,
        _abi: Option<&str>,
    ) -> PyResult<String> {
        let env = build_test_env(self, addr(deployer)?, TransactTo::Create(CreateScheme::Create), code.unwrap_or_default().into(), value.unwrap_or_default());
        match deploy_with_env(self.db(), env)
        {
            Ok((address, state)) => {
                self.db.commit(state);
                Ok(format!("{:?}", address))
            },
            Err(e) => Err(e),
        }
    }

}

/// Creates the environment to use when executing a transaction in a test context
///
/// If using a backend with cheatcodes, `tx.gas_price` and `block.number` will be overwritten by
/// the cheatcode state inbetween calls.
fn build_test_env(
    evm: &EVM,
    caller: Address,
    transact_to: TransactTo,
    data: Bytes,
    value: U256,
) -> EnvWithHandlerCfg {
    let env = revm::primitives::Env {
        cfg: evm.env.cfg.clone(),
        // We always set the gas price to 0, so we can execute the transaction regardless of
        // network conditions - the actual gas price is kept in `evm.block` and is applied by
        // the cheatcode handler if it is enabled
        block: BlockEnv {
            basefee: U256::ZERO,
            gas_limit: evm.gas_limit,
            ..evm.env.block.clone()
        },
        tx: TxEnv {
            caller,
            transact_to,
            data,
            value,
            // As above, we set the gas price to 0.
            gas_price: U256::ZERO,
            gas_priority_fee: None,
            gas_limit: evm.gas_limit.to(),
            ..evm.env.tx.clone()
        },
    };

    EnvWithHandlerCfg::new_with_spec_id(Box::new(env), evm.handler_cfg.spec_id)
}

/// Deploys a contract using the given `env` and commits the new state to the underlying
/// database
fn deploy_with_env(
    db: DB<'_>,
    env: EnvWithHandlerCfg,
) -> PyResult<(Address, State)> {
    debug_assert!(
        matches!(env.tx.transact_to, TransactTo::Create(_)),
        "Expect create transaction"
    );
    trace!(sender=?env.tx.caller, "deploying contract");

    let (result, state) = execute(db, env)?;

    match &result {
        Success { reason, gas_used, gas_refunded, logs, output } => {
            warn!(reason=?reason, gas_used, gas_refunded, "contract deployed");
            match output {
                Output::Create(_, address) => {
                    Ok((address.unwrap(), state))
                }
                _ => Err(utils::pyerr("Invalid output")),
            }
        },
        _ => Err(utils::pyerr(result.clone())),
    }
}

fn execute(db: DB<'_>, env: EnvWithHandlerCfg) -> Result<(ExecutionResult, State), PyErr> {
    let ResultAndState {
        result, state
    } = match db {
        DB::Memory(db) =>
            Evm::builder()
                .with_ref_db(db)
                .with_env_with_handler_cfg(env)
                .build()
                .transact()
                .map_err(utils::pyerr)?,
        DB::Fork(db) => Evm::builder()
            .with_ref_db(db)
            .with_env_with_handler_cfg(env)
            .build()
            .transact()
            .map_err(utils::pyerr)?,
    };
    Ok((result, state))
}


fn call_raw_with_env(
    db: DB<'_>,
    env: EnvWithHandlerCfg,
) -> PyResult<(Bytes, State)> {
    debug_assert!(
        matches!(env.tx.transact_to, TransactTo::Call(_)),
        "Expect call transaction"
    );
    trace!(sender=?env.tx.caller, "deploying contract");

    let (result, state) = execute(db, env)?;

    match &result {
        Success { reason, gas_used, gas_refunded, logs, output } => {
            let data = output.clone().into_data();
            trace!(reason=?reason, gas_used, gas_refunded, "call done");
            Ok((data, state))
        },
        // todo: state might have changed even if the call failed
        _ => Err(utils::pyerr(result.clone())),
    }
}

fn call_raw(
    evm: &EVM,
    caller: &str,
    to: &str,
    calldata: Option<Vec<u8>>,
    value: Option<U256>,
) -> PyResult<(Bytes, State)> {
    let env = build_test_env(&evm, addr(caller)?, TransactTo::Call(addr(to)?), calldata.unwrap_or_default().into(), value.unwrap_or_default().into());
    call_raw_with_env(evm.db(), env)
}
