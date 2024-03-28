use std::collections::HashMap;

mod checkpoint;
pub use checkpoint::*;

mod evm_env;
pub use evm_env::*;

mod execution_result;
pub use execution_result::*;

mod info;
pub use info::*;

// In Py03 we use vec<u8> to represent bytes
pub(crate) type PyByteVec = Vec<u8>;
pub(crate) type PyDB = HashMap<String, AccountInfo>;
