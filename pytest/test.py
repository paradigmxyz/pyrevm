import pytest

from pyrevm import *

address = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
address2 = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"

def test_revm():
    evm = EVM();
    cfg = CfgEnv();
    block = BlockEnv();
    print("BLOCK", block)

    info = AccountInfo(
        balance = 1000000000000000000000000000000000000
    )
    evm.insert_account_info(address, info)

    tx = TxEnv();
    tx.caller = address
    tx.to = address2
    tx.value = 1000000000000000000000000000000000000

    evm.env = Env(cfg, block, tx)
    res = evm.transact_commit();
    print(res)
    evm.dump();
    print(info.balance)
