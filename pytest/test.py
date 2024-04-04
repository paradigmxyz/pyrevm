import json
import os.path

import pytest
from pyrevm import EVM, Env, BlockEnv, AccountInfo

address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"  # vitalik.eth
address2 = "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"

fork_url = "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"

KWARG_CASES = [
    {"fork_url": fork_url},
    {"fork_url": fork_url, "tracing": False, "fork_block": "latest"},
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

    assert evm.env.block.timestamp == 100

    vb_before = evm.basic(address)
    assert vb_before is not None

    # Execute the tx
    evm.message_call(
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

    account_before = evm.basic(address)
    assert account_before is not None
    assert account_before.nonce == 0
    assert account_before.code == b"\0"

    # Deploy the contract
    code = load_contract_bin("blueprint.bin")
    deployed_at = evm.deploy(address, code)

    assert deployed_at == "0x3e4ea2156166390f880071d94458efb098473311"
    deployed_code = evm.get_code(deployed_at)
    assert deployed_code.hex().rstrip('0') in code.hex()
    assert evm.basic(deployed_at).code.hex() == deployed_code.hex()

    result = evm.message_call(
        address,
        deployed_at,
        calldata=b'\xc2\x98Ux'  # ==method_id('foo()')
    )
    assert int(result.hex(), 16) == 123


def test_balances():
    evm = EVM()

    vb_before = evm.basic(address)
    assert vb_before is not None
    assert vb_before.balance == 0

    # Give ether
    amount = 10000
    evm.set_balance(address, amount)

    assert evm.get_balance(address) == amount
    assert evm.basic(address).balance == amount


def test_balances_fork():
    evm = EVM(fork_url=fork_url, fork_block="0x3b01f793ed1923cd82df5fe345b3e12211aedd514c8546e69efd6386dc0c9a97")

    vb_before = evm.basic(address)
    assert vb_before.balance == 955628344913799071315

    amount = 10000
    evm.set_balance(address, amount)

    assert evm.get_balance(address) == amount
    assert evm.basic(address).balance == amount


@pytest.mark.parametrize("kwargs", KWARG_CASES)
def test_message_call(kwargs):
    evm = EVM(**kwargs)
    info = AccountInfo(code=load_contract_bin("full_math.bin"))
    evm.insert_account_info(address, info)
    assert evm.basic(address).code == info.code

    # mulDiv() -> 64 * 8 / 2
    result = evm.message_call(
        caller=address2,
        to=address,
        calldata=bytes.fromhex(
            f"aa9a0912{encode_uint(64)}{encode_uint(8)}{encode_uint(2)}"
        ),
    )

    assert int.from_bytes(result, "big") == 256


@pytest.mark.parametrize("kwargs", KWARG_CASES)
def test_call_committing(kwargs):
    evm = EVM(**kwargs)
    evm.insert_account_info(
        address, AccountInfo(code=load_contract_bin("full_math.bin"))
    )

    # mulDivRoundingUp() -> 64 * 8 / 3
    result = evm.message_call(
        caller=address2,
        to=address,
        calldata=bytes.fromhex(
            f"0af8b27f{encode_uint(64)}{encode_uint(8)}{encode_uint(3)}"
        ),
    )

    assert int.from_bytes(result, "big") == 171


def test_call_revert():
    evm = EVM()
    amount = 10000
    evm.set_balance(address2, amount)

    checkpoint = evm.snapshot()
    evm.message_call(
        caller=address2,
        to=address,
        value=amount,
    )

    assert evm.get_balance(address) == amount
    evm.revert(checkpoint)
    assert evm.get_balance(address) == 0
    assert evm.get_balance(address2) == amount


@pytest.mark.parametrize("kwargs", KWARG_CASES)
def test_call_empty_result(kwargs):
    evm = EVM(**kwargs)
    evm.insert_account_info(address, AccountInfo(code=load_contract_bin("weth_9.bin")))

    evm.set_balance(address2, 10000)

    deposit = evm.message_call(
        caller=address2,
        to=address,
        value=10000,
        calldata=bytes.fromhex("d0e30db0"),
    )

    assert deposit == b""

    balance = evm.message_call(
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
    evm.message_call(
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


def test_blueprint():
    evm = EVM()
    # bytecode based on vyper `@external def foo() -> uint256: return 123`
    bytecode = load_contract_bin("blueprint.bin")

    bytecode = b"\xFE\x71\x00" + bytecode
    bytecode_len = len(bytecode)
    bytecode_len_hex = hex(bytecode_len)[2:].rjust(4, "0")

    # prepend a quick deploy preamble
    deploy_preamble = bytes.fromhex("61" + bytecode_len_hex + "3d81600a3d39f3")
    deploy_bytecode = deploy_preamble + bytecode

    deployer_address = evm.deploy(address, deploy_bytecode)
    assert evm.basic(deployer_address).code.hex().rstrip('0') in deploy_bytecode.hex()
