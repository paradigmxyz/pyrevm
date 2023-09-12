use std::fmt::Debug;

use foundry_evm::{
    executor::{fork::CreateFork, opts::EvmOpts, Backend, Executor, ExecutorBuilder},
    utils::RuntimeOrHandle,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use revm::{primitives::U256, Database};

use crate::{
    types::{AccountInfo, Env},
    utils::addr,
};

#[pyclass]
pub struct EVM(Executor);

impl EVM {
    pub fn db(&self) -> &Backend {
        &self.0.backend
    }
}

fn pyerr<T: Debug>(err: T) -> pyo3::PyErr {
    PyRuntimeError::new_err(format!("{:?}", err))
}

#[pymethods]
impl EVM {
    #[new]
    #[pyo3(signature = (env=None, fork_url=None, fork_block_number=None, gas_limit=18446744073709551615, tracing=false))]
    fn new(
        env: Option<Env>,
        fork_url: Option<String>,
        fork_block_number: Option<u64>,
        gas_limit: u64,
        tracing: bool,
    ) -> PyResult<Self> {
        let evm_opts = EvmOpts {
            fork_url: fork_url.clone(),
            fork_block_number,
            ..Default::default()
        };

        let fork_opts = if let Some(fork_url) = fork_url {
            let env = RuntimeOrHandle::new()
                .block_on(evm_opts.evm_env())
                .map_err(pyerr)?;
            Some(CreateFork {
                url: fork_url,
                enable_caching: true,
                env,
                evm_opts,
            })
        } else {
            None
        };

        let db = RuntimeOrHandle::new().block_on(Backend::spawn(fork_opts));

        let executor = ExecutorBuilder::default()
            .gas_limit(gas_limit.into())
            .inspectors(|stack| stack.trace(tracing))
            .build(env.unwrap_or_default().into(), db);

        Ok(EVM(executor))
    }

    /// Inserts the provided account information in the database at
    /// the specified address.
    fn basic(mut _self: PyRefMut<'_, Self>, address: &str) -> PyResult<Option<AccountInfo>> {
        let db = &mut _self.0.backend;
        let acc = db.basic(addr(address)?).map_err(pyerr)?;
        Ok(acc.map(Into::into))
    }

    /// Inserts the provided account information in the database at
    /// the specified address.
    fn insert_account_info(
        mut _self: PyRefMut<'_, Self>,
        address: &str,
        info: AccountInfo,
    ) -> PyResult<()> {
        let db = &mut _self.0.backend;
        db.insert_account_info(addr(address)?.into(), info.into());

        Ok(())
    }

    /// Set the balance of a given address.
    fn set_balance(mut _self: PyRefMut<'_, Self>, address: &str, balance: U256) -> PyResult<()> {
        _self
            .0
            .set_balance(addr(address)?.into(), balance.into())
            .map_err(pyerr)?;
        Ok(())
    }

    /// Retrieve the balance of a given address.
    fn get_balance(_self: PyRef<'_, Self>, address: &str) -> PyResult<U256> {
        let balance = _self.0.get_balance(addr(address)?.into()).map_err(pyerr)?;
        Ok(balance.into())
    }

    fn call_raw_committing(
        mut _self: PyRefMut<'_, Self>,
        caller: &str,
        to: &str,
        value: Option<U256>,
        data: Option<Vec<u8>>,
    ) -> PyResult<()> {
        let res = _self
            .0
            .call_raw_committing(
                // TODO: The constant type conversions when
                // crossing the boundary is annoying. Can we pass it
                // a type that's already an `Address`?
                addr(caller)?.into(),
                addr(to)?.into(),
                data.unwrap_or_default().into(),
                value.unwrap_or_default().into(),
            )
            .map_err(pyerr)?;

        if res.reverted {
            return Err(pyerr(res.exit_reason));
        }

        // TODO: Return the traces back to the user.
        dbg!(&res.traces);
        Ok(())
    }

    fn call_raw(
        _self: PyRef<'_, Self>,
        caller: &str,
        to: &str,
        value: Option<U256>,
        data: Option<Vec<u8>>,
    ) -> PyResult<()> {
        let res = _self
            .0
            .call_raw(
                addr(caller)?.into(),
                addr(to)?.into(),
                data.unwrap_or_default().into(),
                value.unwrap_or_default().into(),
            )
            .map_err(pyerr)?;

        if res.reverted {
            return Err(pyerr(res.exit_reason));
        }

        dbg!(&res.traces);
        Ok(())
    }

    /// Deploy a contract with the given code.
    fn deploy(
        mut _self: PyRefMut<'_, Self>,
        deployer: &str,
        code: Option<Vec<u8>>,
        value: Option<U256>,
        _abi: Option<&str>,
    ) -> PyResult<String> {
        let res = _self
            .0
            .deploy(
                addr(deployer)?.into(),
                code.unwrap_or_default().into(),
                value.unwrap_or_default().into(),
                None,
            )
            .map_err(pyerr)?;

        dbg!(&res.traces);
        Ok(format!("{:?}", res.address))
    }
}
