import pytest

from pyrevm import *

address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045" # vitalik.eth
address2 = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"

fork_url = "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"

def test_revm():
    # set up an evm
    evm = EVM(
        # can fork from a remote node
        fork_url=fork_url, 
        # can set tracing to true/false
        tracing = True, 
        # can configure the environment
        env = Env(
            block = BlockEnv( timestamp = 100
        ))
    );

    vb_before = evm.basic(address)
    assert vb_before != 0

    # Execute the tx
    evm.call_raw_committing(
        caller = address,
        to = address2,
        value = 10000
        # data
    );

    assert vb_before != evm.basic(address)
    assert evm.basic(address2).balance == 10000
