use std::convert::Infallible;
use revm::{
    db::{DatabaseRef, EmptyDB},
    primitives::{AccountInfo, Bytecode},
};
use revm::primitives::{Address, B256, U256};

/// An empty database that always returns default values when queried.
/// This will also _always_ return `Some(AccountInfo)`.
/// Copied from Foundry: https://github.com/foundry-rs/foundry/blob/9e3ab9b3aff21c6e5ef/crates/evm/core/src/backend/in_memory_db.rs#L83-L92

#[derive(Clone, Debug, Default)]
pub struct EmptyDBWrapper(EmptyDB);

impl DatabaseRef for EmptyDBWrapper {
    type Error = Infallible;

    fn basic_ref(&self, _address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        // Note: this will always return `Some(AccountInfo)`, for the reason explained above
        Ok(Some(AccountInfo::default()))
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.0.code_by_hash_ref(code_hash)?)
    }
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(self.0.storage_ref(address, index)?)
    }

    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        Ok(self.0.block_hash_ref(number)?)
    }
}
