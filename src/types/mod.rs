use std::collections::HashMap;
pub use evm_env::*;
pub use info::*;
pub use execution_result::*;

mod evm_env;
mod info;
mod execution_result;

// In Py03 we use vec<u8> to represent bytes
pub(crate) type PyBytes = Vec<u8>;
pub(crate) type Checkpoint = i32;
pub(crate) type PyDB = HashMap<String, AccountInfo>;
