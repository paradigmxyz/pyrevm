import cProfile
import pathlib
import pstats
from typing import Final

import pyrevm

CONTRACT_DATA_FILE_PATH: Final[pathlib.Path] = (
    pathlib.Path(__file__).absolute().parent / "./snailtracer.evm"
)
ZERO_ADDRESS: Final[str] = "0x0000000000000000000000000000000000000000"
CALLER_ADDRESS: Final[str] = "0x1000000000000000000000000000000000000001"


def _load_contract_data(data_file_path: pathlib.Path) -> bytes:
    with open(data_file_path, mode="r") as file:
        return bytes.fromhex(file.read())


def _construct_evm(contract_address: str, contract_data: bytes) -> pyrevm.EVM:
    evm = pyrevm.EVM()
    evm.insert_account_info(
        contract_address,
        pyrevm.AccountInfo(code=contract_data),
    )
    return evm


def _benchmark(
    evm: pyrevm.EVM,
    caller_address: str,
    contract_address: str,
    call_data: list[int],
    num_runs: int = 10,
    warmup_runs: int = 2,
) -> None:
    def bench() -> None:
        evm.message_call(
            caller=caller_address,
            to=contract_address,
            data=call_data,
        )

    for _ in range(warmup_runs):
        bench()

    with cProfile.Profile() as pr:
        for _ in range(num_runs):
            bench()

        pr.disable()
        p = pstats.Stats(pr)
        p.sort_stats(pstats.SortKey.CUMULATIVE).print_stats(10)


def main() -> None:
    contract_data = _load_contract_data(CONTRACT_DATA_FILE_PATH)
    evm = _construct_evm(ZERO_ADDRESS, contract_data)

    _benchmark(
        evm,
        caller_address=CALLER_ADDRESS,
        contract_address=ZERO_ADDRESS,
        call_data=list(bytes.fromhex("30627b7c")),
        num_runs=10,
        warmup_runs=2,
    )


if __name__ == "__main__":
    main()
