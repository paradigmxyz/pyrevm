import cProfile
import pathlib
import pstats
from typing import Final
import time
import contextlib

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


@contextlib.contextmanager
def timeit(msg, n):
    start_time = time.perf_counter()
    yield
    end_time = time.perf_counter()
    total_time = end_time - start_time
    msg += f" Took {total_time:.4f}s"
    if n != 1:
        per_time = total_time * 1000_000 / n
        msg += f" ({per_time:.4f}us per run)"
    print(msg)


def _benchmark(
    evm: pyrevm.EVM,
    caller_address: str,
    contract_address: str,
    call_data: list[int],
    num_runs: int = 10,
    warmup_runs: int = 2,
    profile=False
) -> None:
    def bench() -> None:
        evm.call_raw(
            caller=caller_address,
            to=contract_address,
            data=call_data,
        )

    for _ in range(warmup_runs):
        bench()

    if profile:
        with cProfile.Profile() as pr:
            for _ in range(num_runs):
                bench()

            pr.disable()
            p = pstats.Stats(pr)
            p.sort_stats(pstats.SortKey.CUMULATIVE).print_stats(10)
    else:
        with timeit(f"bench {num_runs} times", n=num_runs):
            rust_elapsed_nanos = 0
            for _ in range(num_runs):
                rust_elapsed_nanos += evm.call_raw(
                    caller=caller_address,
                    to=contract_address,
                    #data=call_data,  # <- 400nanos
                )
            micros = rust_elapsed_nanos/1000 / num_runs
            print(f"rust elapsed: {micros:.4f}us")


def main() -> None:
    #contract_data = _load_contract_data(CONTRACT_DATA_FILE_PATH)
    bytecode = bytes.fromhex("5F5F5050")  # PUSH0 PUSH0 POP POP
    evm = _construct_evm(ZERO_ADDRESS, bytecode)

    _benchmark(
        evm,
        caller_address=CALLER_ADDRESS,
        contract_address=ZERO_ADDRESS,
        call_data=list(bytes.fromhex("30627b7c")),
        num_runs=1_000_000,
        warmup_runs=2000,
    )


if __name__ == "__main__":
    main()
