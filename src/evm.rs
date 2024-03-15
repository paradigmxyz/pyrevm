use std::collections::btree_map::Entry;
use std::fmt::Debug;

use foundry_evm::executors::RawCallResult;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use revm::{Database, DatabaseCommit, Evm, EvmBuilder, InMemoryDB, Inspector, inspector_handle_register, primitives::U256};
use revm::db::{CacheDB, DbAccount, EmptyDB};
use revm::inspectors::NoOpInspector;
use revm::precompile::{Address, Bytes, Log};
use revm::primitives::{BlockEnv, Bytecode, CreateScheme, Env as RevmEnv, EnvWithHandlerCfg, ExecutionResult, HaltReason, HandlerCfg, Output, ResultAndState, SpecId, SuccessReason, TransactTo, TxEnv};
use revm::primitives::ExecutionResult::{Halt, Revert, Success};
use revm::precompile::B256;
use tracing::{trace, warn};

use crate::{
    types::{AccountInfo, Env},
    utils::addr,
};

#[derive(Clone, Debug)]
#[pyclass]
pub struct EVM {
    /// The underlying `revm::Database` that contains the EVM storage.
    // Note: We do not store an EVM here, since we are really
    // only interested in the database. REVM's `EVM` is a thin
    // wrapper around spawning a new EVM on every call anyway,
    // so the performance difference should be negligible.
    pub db: CacheDB<InMemoryDB>,
    /// The EVM environment.
    pub env: RevmEnv,
    pub handler_cfg: HandlerCfg,
    /// The gas limit for calls and deployments. This is different from the gas limit imposed by
    /// the passed in environment, as those limits are used by the EVM for certain opcodes like
    /// `gaslimit`.
    gas_limit: U256,
}

impl EVM {
    pub fn db(&self) -> &CacheDB<InMemoryDB> {
        &self.db
    }
}

fn pyerr<T: Debug>(err: T) -> PyErr {
    PyRuntimeError::new_err(format!("{:?}", err))
}

#[pymethods]
impl EVM {
    #[new]
    #[pyo3(signature = (env=None, gas_limit=18446744073709551615, tracing=false))]
    fn new(
        env: Option<Env>,
        gas_limit: u64,
        tracing: bool
    ) -> PyResult<Self> {
        // let db = CacheDB::new(EmptyDB::default());
        let db = CacheDB::new(Default::default());
        Ok(EVM {
            // db: CacheDB::new(InMemoryDB::new(EmptyDB::new())),
            db,
            env: env.or(Some(Env::default())).unwrap().into(),
            gas_limit: U256::from(gas_limit),
            handler_cfg: HandlerCfg::new(SpecId::LATEST),
        })
    }

    /// Get basic account information.
    fn basic(mut _self: PyRefMut<'_, Self>, address: &str) -> PyResult<AccountInfo> {
        let acc = _self.db.basic(addr(address)?).map_err(pyerr)?;
        Ok(acc.map(Into::into).unwrap_or_else(AccountInfo::default))
    }

    /// Get account code by its hash.
    #[pyo3(signature = (code_hash))]
    fn code_by_hash(&mut self, code_hash: &str) -> PyResult<Vec<u8>> {
        let hash = code_hash.parse::<B256>().map_err(pyerr)?;
        Ok(self.db.code_by_hash(hash).map(|c| c.bytecode.to_vec()).map_err(pyerr)?)
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: &str, index: U256) -> PyResult<U256> {
        Ok(self.db.storage(addr(address)?, index).map_err(pyerr)?)
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: U256) -> PyResult<Vec<u8>> {
        Ok(self.db.block_hash(number).map(|h| h.to_vec()).map_err(pyerr)?)
    }

    /// Inserts the provided account information in the database at the specified address.
    fn insert_account_info(
        mut _self: PyRefMut<'_, Self>,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        _self.db.insert_account_info(addr(address)?, info.into());
        Ok(())
    }

    /// Set the balance of a given address.
    fn set_balance(mut _self: PyRefMut<'_, Self>, address: &str, balance: U256) -> PyResult<()> {
        let target = addr(address)?;
        let mut info = _self.db.basic(target).map_err(pyerr)?.unwrap_or_default();
        info.balance = balance;
        _self.db.insert_account_info(target, info.clone());
        if !_self.db.accounts.contains_key(&target) {
            panic!("Account not found after insert");
        }
        // assert_eq!(_self.db.accounts.entry(target).map(|o| o.map(|b| b.info)), Some(info));
        // assert_eq!(_self.db.basic(target).map_err(pyerr)?, Entry::Occupied(info));
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(mut _self: PyRefMut<'_, Self>, address: &str) -> PyResult<Option<U256>> {
        // Ok(_self.db.basic(addr(address).map_err(pyerr)?)?.map(|acc| acc.balance))
        let acc = _self.db.basic(addr(address)?).map_err(pyerr)?;
        Ok(acc.map(|a| a.balance))
    }

    // runs a raw call and returns the result
    #[pyo3(signature = (caller, to, calldata=None, value=None))]
    pub fn call_raw_committing(
        mut _self: PyRefMut<'_, Self>,
        caller: &str,
        to: &str,
        calldata: Option<Vec<u8>>,
        value: Option<U256>,
    ) -> PyResult<Vec<u8>> {
        call_raw(_self, caller, to, calldata, value, true)
    }

    #[pyo3(signature = (caller, to, calldata=None, value=None, commit=false))]
    pub fn call_raw(
        mut _self: PyRefMut<'_, Self>,
        caller: &str,
        to: &str,
        calldata: Option<Vec<u8>>,
        value: Option<U256>,
        commit: bool,
    ) -> PyResult<Vec<u8>> {
        call_raw(_self, caller, to, calldata, value, commit)
    }

    /// Deploy a contract with the given code.
    fn deploy(
        mut _self: PyRefMut<'_, Self>,
        deployer: &str,
        code: Option<Vec<u8>>,
        value: Option<U256>,
        _abi: Option<&str>,
    ) -> PyResult<String> {
        let env = build_test_env(&_self, addr(deployer)?, TransactTo::Create(CreateScheme::Create), code.unwrap_or_default().into(), value.unwrap_or_default());
        let address = deploy_with_env(_self, env)
            .map_err(pyerr)?;
        Ok(format!("{:?}", address))
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
    mut _self: PyRefMut<'_, EVM>,
    env: EnvWithHandlerCfg,
) -> PyResult<Address> {
    debug_assert!(
        matches!(env.tx.transact_to, TransactTo::Create(_)),
        "Expect create transaction"
    );
    trace!(sender=?env.tx.caller, "deploying contract");

    let ResultAndState {
        result, state
    } = Evm::builder()
        .with_ref_db(&_self.db)
        .with_env_with_handler_cfg(env)
        .build()
        .transact()
        .map_err(pyerr)?;

    match &result {
        Success { reason, gas_used, gas_refunded, logs, output } => {
            warn!(reason=?reason, gas_used, gas_refunded, "contract deployed");
            match output {
                Output::Create(_, address) => {
                    _self.db.commit(state);
                    Ok(address.unwrap())
                }
                _ => Err(pyerr("Invalid output")),
            }
        },
        _ => Err(pyerr(result.clone())),
    }
}


fn call_raw_with_env(
    mut _self: PyRefMut<'_, EVM>,
    env: EnvWithHandlerCfg,
) -> PyResult<Bytes> {
    debug_assert!(
        matches!(env.tx.transact_to, TransactTo::Call(_)),
        "Expect call transaction"
    );
    trace!(sender=?env.tx.caller, "deploying contract");

    let transaction = Evm::builder()
        .with_env_with_handler_cfg(env)
        .build()
        .transact()
        .map_err(pyerr)?;

    let ResultAndState {
        result, state
    } = transaction;

    match &result {
        Success { reason, gas_used, gas_refunded, logs, output } => {
            let data = output.clone().into_data();
            trace!(reason=?reason, gas_used, gas_refunded, "call done");
            Ok(data)
        },
        _ => Err(pyerr(result.clone())),
    }
}

fn call_raw(
    mut _self: PyRefMut<'_, EVM>,
    caller: &str,
    to: &str,
    calldata: Option<Vec<u8>>,
    value: Option<U256>,
    commit: bool,
) -> PyResult<Vec<u8>> {
    let env = build_test_env(&_self, addr(caller)?, TransactTo::Call(addr(to)?), calldata.unwrap_or_default().into(), value.unwrap_or_default().into());
    let mut result = call_raw_with_env(_self, env).map_err(pyerr)?;
    // self.commit(&mut result);
    // TODO: Return the traces back to the user.
    Ok(result.to_vec())
}
