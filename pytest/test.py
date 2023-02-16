import os.path

import pytest

from pyrevm import *

address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"  # vitalik.eth
address2 = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"

fork_url = "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"


def load_contract_bin(contract_name: str) -> bytes:
    with open(f"{os.path.dirname(__file__)}/contracts/{contract_name}", "r") as readfile:
        hexstring = readfile.readline()
    return bytes.fromhex(hexstring)


def encode_uint(num: int):
    encoded = hex(num)[2:]
    return ("0" * (64 - len(encoded))) + encoded


def test_revm():
    # set up an evm
    evm = EVM(
        # can fork from a remote node
        fork_url=fork_url,
        # can set tracing to true/false
        tracing=True,
        # can configure the environment
        env=Env(block=BlockEnv(timestamp=100)),
    )

    vb_before = evm.basic(address)
    assert vb_before is not None

    # Execute the tx
    evm.call_raw_committing(
        caller=address,
        to=address2,
        value=10000
        # data
    )

    info = evm.basic(address2)

    assert info is not None
    assert vb_before != evm.basic(address)
    assert info.balance == 10000


def test_deploy():
    evm = EVM()

    vb_before = evm.basic(address)
    assert vb_before is not None
    assert vb_before.nonce == 0

    # Deploy the contract
    deployed_at = evm.deploy(
        deployer=address,
        code=list(bytes.fromhex("6060")),
    )

    assert deployed_at == "0x3e4ea2156166390f880071d94458efb098473311"


def test_balances():
    evm = EVM()

    vb_before = evm.basic(address)
    assert vb_before is not None
    assert vb_before.balance == 0

    # Give ether
    AMT = 10000
    evm.set_balance(address, AMT)

    assert evm.get_balance(address) == AMT
    assert evm.basic(address).balance == AMT


def test_call_raw():
    evm = EVM()
    evm.insert_account_info(address, AccountInfo(code=load_contract_bin("full_math.bin")))

    # mulDiv() -> 64 * 8 / 2
    result = evm.call_raw(caller=address2, to=address, data=bytes.fromhex(f"aa9a0912{encode_uint(64)}{encode_uint(8)}{encode_uint(2)}"))

    assert int(result, 16) == 256


def test_call_committing():
    evm = EVM()
    evm.insert_account_info(address, AccountInfo(code=load_contract_bin("full_math.bin")))

    # mulDivRoundingUp() -> 64 * 8 / 3
    result = evm.call_raw(caller=address2, to=address, data=bytes.fromhex(f"0af8b27f{encode_uint(64)}{encode_uint(8)}{encode_uint(3)}"))

    assert int(result, 16) == 171
