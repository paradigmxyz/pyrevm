from pyrevm import EVM, BlockEnv, TxEnv, fake_exponential

from tests.utils import load_contract_bin


DEPLOYER = "0x1111111111111111111111111111111111111111"
MIN_BLOB_BASE_FEE = 1
BLOB_BASE_FEE_UPDATE_FRACTION = 3_338_477
VERSIONED_HASH_VERSION_KZG = b"\x01"


def test_blob_base_fee():
    evm = EVM()
    evm.set_block_env(BlockEnv(excess_blob_gas=10**6))

    deploy_address = evm.deploy(DEPLOYER, load_contract_bin("blob_base_fee.bin"))

    # hash of b"Vyper the language of the sneks!".rjust(32 * 4096)
    blob_hashes = 6 * [bytes.fromhex("01e378e5cef6f2a88abd923440aae7ce210414a610233aa457100f20f884d0de")]
    evm.set_tx_env(TxEnv(max_fee_per_blob_gas=10**10, blob_hashes=blob_hashes))
    assert evm.env.block.blob_gasprice == MIN_BLOB_BASE_FEE

    evm.set_balance(DEPLOYER, 10**20)
    for _i in range(10):
        evm.message_call(
            caller=DEPLOYER,
            to="0xb45BEc6eeCA2a09f4689Dd308F550Ad7855051B5",  # random address
            gas=21000,
        )
        result = evm.message_call(
            caller=DEPLOYER,
            to=deploy_address,
            calldata=b'_\xb0\x14m',  # == method_id("get_blobbasefee()")
        )
        expected = fake_exponential(MIN_BLOB_BASE_FEE, evm.env.block.blob_gasprice, BLOB_BASE_FEE_UPDATE_FRACTION)
        assert int.from_bytes(result, "big") == expected

    # sanity check that blobbasefee has increased above the minimum
    assert evm.env.block.excess_blob_gas > MIN_BLOB_BASE_FEE
