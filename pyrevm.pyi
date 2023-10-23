from typing import Optional, Type

class CfgEnv:
    def __new__(cls: Type["CfgEnv"]) -> "CfgEnv": ...

class BlockEnv:
    def __new__(
        cls: Type["BlockEnv"],
        number: Optional[int] = None,
        coinbase: Optional[str] = None,
        timestamp: Optional[int] = None,
        difficulty: Optional[int] = None,
        prevrandao: Optional[bytes] = None,
        basefee: Optional[int] = None,
        gas_limit: Optional[int] = None,
    ) -> "BlockEnv": ...

class TxEnv:
    def __new__(
        cls: Type["TxEnv"],
        caller: Optional[str] = None,
        gas_limit: Optional[int] = None,
        gas_price: Optional[int] = None,
        gas_priority_fee: Optional[int] = None,
        to: Optional[str] = None,
        value: Optional[int] = None,
        data: Optional[list[int]] = None,
        chain_id: Optional[int] = None,
        nonce: Optional[int] = None,
    ) -> "TxEnv": ...

class Env:
    def __new__(
        cls: Type["Env"],
        cfg: Optional[CfgEnv] = None,
        block: Optional[BlockEnv] = None,
        tx: Optional[TxEnv] = None,
    ) -> "Env": ...

class AccountInfo:
    @property
    def balance(self: "AccountInfo") -> int: ...
    @property
    def nonce(self: "AccountInfo") -> int: ...
    @property
    def code(self: "AccountInfo") -> list[int]: ...
    @property
    def code_hash(self: "AccountInfo") -> list[int]: ...
    def __new__(
        cls: Type["AccountInfo"],
        nonce: int = 0,
        code_hash: Optional[list[int]] = None,
        code: Optional[list[int]] = None,
    ) -> "AccountInfo": ...

class EvmOpts:
    env: Env
    fork_url: Optional[str]
    fork_block_number: Optional[int]
    gas_limit: int
    tracing: bool
    def __new__(
        cls: Type["EvmOpts"],
        env: Optional[Env],
        fork_url: Optional[str],
    ) -> "EvmOpts": ...

class EVM:
    def __new__(
        cls: Type["EVM"],
        env: Optional[Env] = None,
        fork_url: Optional[str] = None,
        fork_block_number: Optional[int] = None,
        gas_limit: int = 2**64 - 1,
        tracing: bool = False,
    ) -> "EVM": ...
    def basic(self: "EVM", address: str) -> Optional[AccountInfo]: ...
    def insert_account_info(self: "EVM", address: str, info: AccountInfo) -> None: ...
    def call_raw_committing(
        self: "EVM",
        caller: str,
        to: str,
        value: Optional[int] = None,
        data: Optional[list[int]] = None,
    ) -> list[int]: ...
    def call_raw(
        self: "EVM",
        caller: str,
        to: str,
        value: Optional[int] = None,
        data: Optional[list[int]] = None,
    ) -> list[int]: ...
    def deploy(
        self: "EVM",
        deployer: str,
        code: list[int],
        value: Optional[int] = None,
    ) -> str: ...
    def get_balance(self: "EVM", address: str) -> int: ...
    def set_balance(self: "EVM", address: str, balance: int) -> None: ...
