import json
import os.path

import pytest
from pyrevm import EVM, Env, BlockEnv, AccountInfo

address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"  # vitalik.eth
address2 = "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"

fork_url = "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"

KWARG_CASES = [
    {"fork_url": fork_url},
    {"fork_url": fork_url, "tracing": False, "fork_block_number": "latest"},
    {},
]


def load_contract_bin(contract_name: str) -> bytes:
    with open(
        f"{os.path.dirname(__file__)}/contracts/{contract_name}", "r"
    ) as readfile:
        hexstring = readfile.readline()
    return bytes.fromhex(hexstring)


def encode_uint(num: int) -> str:
    encoded = hex(num)[2:]
    return ("0" * (64 - len(encoded))) + encoded


def encode_address(address: str) -> str:
    return f'{"0" * 24}{address[2:]}'


def test_revm_fork():
    # set up an evm
    evm = EVM(
        # can fork from a remote node
        fork_url=fork_url,
        # can set tracing to true/false
        tracing=True,
        # can configure the environment
        env=Env(block=BlockEnv(timestamp=100, prevrandao=bytes([0] * 32))),
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
        code=bytes.fromhex("6060"),
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


def test_balances_fork():
    evm = EVM(fork_url=fork_url, fork_block_number="0x3b01f793ed1923cd82df5fe345b3e12211aedd514c8546e69efd6386dc0c9a97")

    vb_before = evm.basic(address)
    assert vb_before.balance == 955628344913799071315

    AMT = 10000
    evm.set_balance(address, AMT)

    assert evm.get_balance(address) == AMT
    assert evm.basic(address).balance == AMT


@pytest.mark.parametrize("kwargs", KWARG_CASES)
def test_call_raw(kwargs):
    evm = EVM(**kwargs)
    info = AccountInfo(code=load_contract_bin("full_math.bin"))
    evm.insert_account_info(address, info)
    assert evm.basic(address).code == info.code

    # mulDiv() -> 64 * 8 / 2
    result, changes = evm.call_raw(
        caller=address2,
        to=address,
        calldata=bytes.fromhex(
            f"aa9a0912{encode_uint(64)}{encode_uint(8)}{encode_uint(2)}"
        ),
    )

    assert int.from_bytes(result, "big") == 256
    assert changes[address].nonce == 0
    assert changes[address2].nonce == 1


@pytest.mark.parametrize("kwargs", KWARG_CASES)
def test_call_committing(kwargs):
    evm = EVM(**kwargs)
    evm.insert_account_info(
        address, AccountInfo(code=load_contract_bin("full_math.bin"))
    )

    # mulDivRoundingUp() -> 64 * 8 / 3
    result = evm.call_raw_committing(
        caller=address2,
        to=address,
        calldata=bytes.fromhex(
            f"0af8b27f{encode_uint(64)}{encode_uint(8)}{encode_uint(3)}"
        ),
    )

    assert int.from_bytes(result, "big") == 171


@pytest.mark.parametrize("kwargs", KWARG_CASES)
def test_call_empty_result(kwargs):
    evm = EVM(**kwargs)
    evm.insert_account_info(address, AccountInfo(code=load_contract_bin("weth_9.bin")))

    evm.set_balance(address2, 10000)

    deposit = evm.call_raw_committing(
        caller=address2,
        to=address,
        value=10000,
        calldata=bytes.fromhex("d0e30db0"),
    )

    assert deposit == []

    balance, _ = evm.call_raw(
        caller=address2,
        to=address,
        calldata=bytes.fromhex("70a08231" + encode_address(address2)),
    )

    assert int.from_bytes(balance, "big") == 10000
    assert not evm.tracing


def test_tracing(capsys):
    evm = EVM(tracing=True)
    evm.insert_account_info(address, AccountInfo(code=load_contract_bin("weth_9.bin")))
    evm.set_balance(address2, 10000)
    evm.call_raw_committing(
        caller=address2,
        to=address,
        value=10000,
        calldata=bytes.fromhex("d0e30db0"),
    )
    assert evm.tracing
    captured = capsys.readouterr()
    traces = [json.loads(i) for i in captured.out.split("\n") if i]
    assert {'gasUsed': '0xffffffffffff5011',
            'output': '0x',
            'pass': True,
            'stateRoot': '0x0000000000000000000000000000000000000000000000000000000000000000'} == traces[-1]
    assert len(traces) == 128
