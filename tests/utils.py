import os

ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"  # vitalik.eth


def load_contract_bin(contract_name: str) -> bytes:
    with open(
            f"{os.path.dirname(__file__)}/fixtures/{contract_name}", "r"
    ) as readfile:
        hexstring = readfile.readline()
    return bytes.fromhex(hexstring)


def encode_uint(num: int) -> str:
    encoded = hex(num)[2:]
    return ("0" * (64 - len(encoded))) + encoded


def encode_address(address: str) -> str:
    return f'{"0" * 24}{address[2:]}'
