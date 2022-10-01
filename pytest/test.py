import pytest

from pyrevm import *

def test_revm():
    evm = EVM();
    block = BlockEnv(gas_limit = 9999999);
    address = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    address2 = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"

    info = {
        "balance": int(1e36),
    }
    evm.insert_account(address, info)

    tx = TxEnv();
    tx.caller = address
    tx.to = address2 # invalid addr will type error
    tx.value = int(1e36) # -1 will type error
    cfg = CfgEnv();
    evm.env = Env(cfg, block, tx)
    res = evm.transact_commit();
    print(res)
    evm.dump();
