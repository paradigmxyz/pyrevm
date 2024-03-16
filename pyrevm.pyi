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

    @property
    def number(self: "BlockEnv") -> int: ...
    @property
    def coinbase(self: "BlockEnv") -> str: ...
    @property
    def timestamp(self: "BlockEnv") -> int: ...
    @property
    def difficulty(self: "BlockEnv") -> int: ...
    @property
    def prevrandao(self: "BlockEnv") -> bytes: ...
    @property
    def basefee(self: "BlockEnv") -> int: ...
    @property
    def gas_limit(self: "BlockEnv") -> int: ...

class TxEnv:
    def __new__(
        cls: Type["TxEnv"],
        caller: Optional[str] = None,
        gas_limit: Optional[int] = None,
        gas_price: Optional[int] = None,
        gas_priority_fee: Optional[int] = None,
        to: Optional[str] = None,
        value: Optional[int] = None,
        data: Optional[bytes] = None,
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
    def code(self: "AccountInfo") -> bytes: ...
    @property
    def code_hash(self: "AccountInfo") -> bytes: ...
    def __new__(
        cls: Type["AccountInfo"],
        nonce: int = 0,
        code_hash: Optional[bytes] = None,
        code: Optional[bytes] = None,
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
        # fork_url: Optional[str] = None,
        # fork_block_number: Optional[int] = None,
        gas_limit: int = 2**64 - 1,
        tracing: bool = False,
        spec_id="latest",
    ) -> "EVM":
        """
        Creates a new EVM instance.
        :param env: The environment.
        :param gas_limit: The gas limit.
        :param tracing: Whether to enable tracing.
        :param spec_id: The spec ID.
        """

    def get_accounts(self) -> dict[str, AccountInfo]:
        """
        :return: a dictionary of account addresses to account info
        """

    def basic(self: "EVM", address: str) -> AccountInfo:
        """
        Returns the basic account info for the given address.
        :param address: The address of the account.
        :return: The account info.
        """

    def insert_account_info(self: "EVM", address: str, info: AccountInfo) -> None:
        """
        Inserts the given account info into the state.
        :param address: The address of the account.
        :param info: The account info.
        """

    def call_raw_committing(
        self: "EVM",
        caller: str,
        to: str,
        calldata: Optional[bytes] = None,
        value: Optional[int] = None,
    ) -> bytes:
        """
        Processes a raw call, committing the result to the state.
        :param caller: The address of the caller.
        :param to: The address of the callee.
        :param calldata: The calldata.
        :param value: The value.
        :return: The return data.
        """

    def call_raw(
        self: "EVM",
        caller: str,
        to: str,
        calldata: Optional[bytes] = None,
        value: Optional[int] = None,
    ) -> bytes:
        """
        Processes a raw call, without committing the result to the state.
        :param caller: The address of the caller.
        :param to: The address of the callee.
        :param calldata: The calldata.
        :param value: The value.
        :return: The return data.
        """

    def deploy(
        self: "EVM",
        deployer: str,
        code: bytes,
        value: Optional[int] = None,
    ) -> str: ...
    def get_balance(self: "EVM", address: str) -> int: ...
    def set_balance(self: "EVM", address: str, balance: int) -> None: ...
