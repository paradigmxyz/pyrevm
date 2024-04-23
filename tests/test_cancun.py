import os
from hashlib import sha256

from ckzg import blob_to_kzg_commitment, load_trusted_setup

from pyrevm import EVM, BlockEnv, TxEnv, fake_exponential

from tests.utils import load_contract_bin


DEPLOYER = "0x1111111111111111111111111111111111111111"
TRUSTED_SETUP = os.path.join(os.path.dirname(__file__), "fixtures/kzg_trusted_setup.txt")
MIN_BLOB_BASE_FEE = 1
BLOB_BASE_FEE_UPDATE_FRACTION = 3_338_477
VERSIONED_HASH_VERSION_KZG = b"\x01"


def _ckg_hash(blob: bytes) -> bytes:
    commitment = blob_to_kzg_commitment(blob, load_trusted_setup(TRUSTED_SETUP))
    return VERSIONED_HASH_VERSION_KZG + sha256(commitment).digest()[1:]


def test_blob_base_fee():
    evm = EVM()
    evm.set_block_env(BlockEnv(excess_blob_gas=10**6))

    deploy_address = evm.deploy(DEPLOYER, load_contract_bin("blob_base_fee.bin"))

    blobs = [b"Vyper the language of the sneks!".rjust(32 * 4096)] * 6
    evm.set_tx_env(TxEnv(max_fee_per_blob_gas=10**10, blob_hashes=[_ckg_hash(b) for b in blobs]))
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
