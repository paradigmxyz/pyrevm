import pytest

from pyrevm import *

address = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
address2 = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"

def test_revm():
    # TODO: Make DB configurable so that we can provide
    # forking providers.
    evm = EVM();

    # Prepare the tx
    evm.env = Env(tx = TxEnv(
        caller = address,
        to = address2,
        value = 500000000000000000000000000000000000
    ))

    # Fund the account with some wei
    info = AccountInfo(
        balance = 1000000000000000000000000000000000000
    )
    evm.insert_account_info(address, info)

    # Execute hte tx
    res = evm.transact_commit();
    print(res)
    evm.dump();

    print(evm.basic(address))
    print(evm.basic(address2))
