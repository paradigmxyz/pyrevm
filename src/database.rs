use std::str::FromStr;
use std::sync::Arc;

use ethers_core::types::BlockId;
use ethers_providers::{Http, Provider};
use pyo3::{PyErr, PyResult};
use revm::Database;
use revm::db::{CacheDB, EthersDB};
use revm::precompile::{Address, B256};
use revm::primitives::{AccountInfo, Bytecode, State};
use revm_interpreter::primitives::db::{DatabaseCommit, DatabaseRef};
use ruint::aliases::U256;

use crate::empty_db_wrapper::EmptyDBWrapper;
use crate::utils::pyerr;

type MemDB = CacheDB<EmptyDBWrapper>;
type ForkDB = CacheDB<EthersDB<Provider<Http>>>;


/// A wrapper around the `CacheDB` and `EthersDB` to provide a common interface
/// without needing dynamic lifetime and generic parameters (unsupported in PyO3)
#[derive(Clone, Debug)]
pub enum DB {
    Memory(Box<MemDB>),
    Fork(Box<ForkDB>),
}

impl DB {
    pub fn new_memory() -> Self {
        DB::Memory(Box::new(MemDB::new(EmptyDBWrapper::default())))
    }

    pub fn new_fork(
        fork_url: &str,
        fork_block_number: Option<&str>,
    ) -> PyResult<Self> {
        let provider = Provider::<Http>::try_from(fork_url).map_err(pyerr)?;
        let block = fork_block_number.map(|n| BlockId::from_str(n)).map_or(Ok(None), |v| v.map(Some)).map_err(pyerr)?;
        let db = EthersDB::new(Arc::new(provider), block).unwrap();
        Ok(DB::Fork(Box::new(CacheDB::new(db))))
    }

    /// Insert account info but not override storage
    pub fn insert_account_info(&mut self, address: Address, info: AccountInfo) {
        match self {
            DB::Memory(db) => db.insert_account_info(address, info),
            DB::Fork(db) => db.insert_account_info(address, info),
        }
    }
}

impl Database for DB {
    type Error = PyErr;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.basic(address).map_err(pyerr)?,
            DB::Fork(db) => db.basic(address).map_err(pyerr)?,
        })
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.code_by_hash(code_hash).map_err(pyerr)?,
            DB::Fork(db) => db.code_by_hash(code_hash).map_err(pyerr)?,
        })
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.storage(address, index).map_err(pyerr)?,
            DB::Fork(db) => db.storage(address, index).map_err(pyerr)?,
        })
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.block_hash(number).map_err(pyerr)?,
            DB::Fork(db) => db.block_hash(number).map_err(pyerr)?,
        })
    }
}

impl DatabaseCommit for DB {
    fn commit(&mut self, changes: State) {
        match self {
            DB::Memory(ref mut db) => db.commit(changes),
            DB::Fork(ref mut db) => db.commit(changes),
        }
    }
}

impl DatabaseRef for DB {
    type Error = PyErr;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.basic_ref(address).map_err(pyerr)?,
            DB::Fork(db) => db.basic_ref(address).map_err(pyerr)?,
        })
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.code_by_hash_ref(code_hash).map_err(pyerr)?,
            DB::Fork(db) => db.code_by_hash_ref(code_hash).map_err(pyerr)?,
        })
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.storage_ref(address, index).map_err(pyerr)?,
            DB::Fork(db) => db.storage_ref(address, index).map_err(pyerr)?,
        })
    }

    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        Ok(match self {
            DB::Memory(db) => db.block_hash_ref(number).map_err(pyerr)?,
            DB::Fork(db) => db.block_hash_ref(number).map_err(pyerr)?,
        })
    }
}