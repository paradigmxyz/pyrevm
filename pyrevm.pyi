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
        excess_blob_gas: Optional[int] = None,
    ) -> "BlockEnv": ...

    @property
    def number(self) -> Optional[int]: ...
    @property
    def coinbase(self) -> Optional[str]: ...
    @property
    def timestamp(self) -> Optional[int]: ...
    @property
    def difficulty(self) -> Optional[int]: ...
    @property
    def prevrandao(self) -> Optional[bytes]: ...
    @property
    def basefee(self) -> Optional[int]: ...
    @property
    def gas_limit(self) -> Optional[int]: ...
    @property
    def excess_blob_gas(self) -> Optional[int]: ...
    @property
    def blob_gasprice(self) -> Optional[int]: ...
    @number.setter
    def number(self, value: Optional[int]) -> None: ...
    @timestamp.setter
    def timestamp(self, value: Optional[int]) -> None: ...
    @excess_blob_gas.setter
    def excess_blob_gas(self, value: Optional[int]) -> None: ...

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
        salt: Optional[int] = None,
        access_list: Optional[list[tuple[str, list[int]]]] = None,
        blob_hashes: Optional[list[bytes]] = None,
        max_fee_per_blob_gas: Optional[int] = None,
    ) -> "TxEnv": ...

    @property
    def caller(self) -> Optional[str]: ...
    @property
    def gas_limit(self) -> Optional[int]: ...
    @property
    def gas_price(self) -> Optional[int]: ...
    @property
    def gas_priority_fee(self) -> Optional[int]: ...
    @property
    def to(self) -> Optional[str]: ...
    @property
    def value(self) -> Optional[int]: ...
    @property
    def data(self) -> Optional[bytes]: ...
    @property
    def chain_id(self) -> Optional[int]: ...
    @property
    def nonce(self) -> Optional[int]: ...
    @property
    def salt(self) -> Optional[int]: ...
    @property
    def access_list(self) -> list[tuple[str, list[int]]]: ...
    @property
    def blob_hashes(self) -> list[bytes]: ...
    @property
    def max_fee_per_blob_gas(self) -> Optional[int]: ...
    @blob_hashes.setter
    def blob_hashes(self, value: list[bytes]) -> None: ...
    @max_fee_per_blob_gas.setter
    def max_fee_per_blob_gas(self, value: Optional[int]) -> None: ...

class Env:
    def __new__(
        cls: Type["Env"],
        cfg: Optional[CfgEnv] = None,
        block: Optional[BlockEnv] = None,
        tx: Optional[TxEnv] = None,
    ) -> "Env": ...
    @property
    def cfg(self: "AccountInfo") -> Optional[CfgEnv]: ...
    @property
    def block(self: "AccountInfo") -> Optional[BlockEnv]: ...
    @property
    def tx(self: "AccountInfo") -> Optional[TxEnv]: ...


class JournalCheckpoint:
    @property
    def log_i(self) -> int: ...
    @property
    def journal_i(self) -> int: ...


class AccountInfo:
    def __new__(
        cls: Type["AccountInfo"],
        nonce: int = 0,
        code_hash: Optional[bytes] = None,
        code: Optional[bytes] = None,
    ) -> "AccountInfo": ...

    @property
    def balance(self: "AccountInfo") -> int: ...
    @property
    def nonce(self: "AccountInfo") -> int: ...
    @property
    def code(self: "AccountInfo") -> Optional[bytes]: ...
    @property
    def code_hash(self: "AccountInfo") -> Optional[bytes]: ...


class ExecutionResult:
    def __new__(
        cls: Type["ExecutionResult"],
        is_success: bool,
        is_halt: bool,
        reason: str,
        gas_used: int,
        gas_refunded: int,
    ) -> "ExecutionResult": ...
    @property
    def is_success(self) -> bool: ...
    @property
    def is_halt(self) -> bool: ...
    @property
    def reason(self) -> str: ...
    @property
    def gas_used(self) -> int: ...
    @property
    def gas_refunded(self) -> int: ...
    @property
    def logs(self) -> list["Log"]: ...


class EVM:
    def __new__(
        cls: Type["EVM"],
        env: Optional[Env] = None,
        fork_url: Optional[str] = None,
        fork_block: Optional[str] = None,
        gas_limit: int = 2**64 - 1,
        tracing: bool = False,
        spec_id="SHANGHAI",
    ) -> "EVM":
        """
        Creates a new EVM instance.
        :param env: The environment.
        :param fork_url: The fork URL.
        :param fork_block: The fork block number. Either a block hash starting with 0x or a block number:
            Supported block numbers: Latest, Finalized, Safe, Earliest, Pending
        :param gas_limit: The gas limit.
        :param tracing: Whether to enable tracing.
        :param spec_id: The spec ID.
        """

    def snapshot(self: "EVM") -> JournalCheckpoint: ...

    def revert(self: "EVM", checkpoint: JournalCheckpoint) -> None: ...
    def commit(self: "EVM") -> None: ...

    def basic(self: "EVM", address: str) -> AccountInfo:
        """
        Returns the basic account info for the given address.
        :param address: The address of the account.
        :return: The account info.
        """

    def get_code(self: "EVM", address: str) -> Optional[bytes]:
        """
        Returns the code of the given address.
        :param address: The address.
        :return: The code.
        """

    def insert_account_info(self: "EVM", address: str, info: AccountInfo) -> None:
        """
        Inserts the given account info into the state.
        :param address: The address of the account.
        :param info: The account info.
        """

    def message_call(
        self: "EVM",
        caller: str,
        to: str,
        calldata: Optional[bytes] = None,
        value: Optional[int] = None,
        gas: Optional[int] = None,
        gas_price: Optional[int] = None,
        is_static = False,
    ) -> bytes:
        """
        Processes a raw call, without committing the result to the state.
        :param caller: The address of the caller.
        :param to: The address of the callee.
        :param calldata: The calldata.
        :param value: The value to be transferred.
        :param gas: The gas supplied for the call.
        :param gas_price: The gas price for the call. Defaults to 0.
        :param is_static: Whether the call is static (i.e. does not change the state).
        :return: The return data and a list of changes to the state.
        """

    def deploy(
        self: "EVM",
        deployer: str,
        code: bytes,
        value: Optional[int] = None,
        gas: Optional[int] = None,
        is_static = False,
        _abi: Optional[list[dict]] = None
    ) -> str:
        """
        Deploys the given code.
        :param deployer: The address of the deployer.
        :param code: The code.
        :param value: The value.
        :param gas: The gas.
        :param is_static: Whether the deployment is static (i.e. does not change the state).
        :param _abi: The ABI.
        :return: The address of the deployed contract.
        """

    def get_balance(self: "EVM", address: str) -> int:
        """
        Returns the balance of the given address.
        :param address: The address.
        :return: The balance.
        """

    def set_balance(self: "EVM", address: str, balance: int) -> None:
        """
        Sets the balance of the given address.
        :param address: The address.
        :param balance: The balance.
        """

    def storage(self: "EVM", address: str, index: int) -> int:
        """
        Returns the storage value of the given address at the given index.
        :param address: The address.
        :param index: The index.
        :return: The storage value.
        """

    def block_hash(self: "EVM", number: int) -> bytes:
        """
        Returns the block hash of the given number.
        :param number: The number.
        :return: The block hash.
        """

    @property
    def env(self: "EVM") -> Env:
        """ Get the environment. """

    @property
    def tracing(self: "EVM") -> bool:
        """ Whether tracing is enabled. """

    @tracing.setter
    def set_tracing(self: "EVM", value: bool) -> None:
        """ Set whether tracing is enabled. """

    @property
    def result(self: "EVM") -> Optional[ExecutionResult]:
        """ The result of the execution. """

    @property
    def checkpoint_ids(self: "EVM") -> set[JournalCheckpoint]:
        """ The checkpoint IDs. """

    @property
    def journal_depth(self: "EVM") -> int:
        """ The journal depth. """

    @property
    def journal_len(self: "EVM") -> int:
        """ The journal length. """

    @property
    def journal_str(self: "EVM") -> str:
        """ The journal string. """

    @property
    def db_accounts(self: "EVM") -> dict[str, AccountInfo]:
        """ The accounts in the database. """

    @property
    def journal_state(self: "EVM") -> dict[str, AccountInfo]:
        """ The state in the journal. """

    def set_block_env(self: "EVM", block: BlockEnv) -> None:
        """ Set the block environment. """

    def set_tx_env(self: "EVM", block: TxEnv) -> None:
        """ Set the transaction environment. """

    def reset_transient_storage(self: "EVM") -> None:
        """ Reset the transient storage. """


class Log:
    @property
    def address(self) -> str: ...

    @property
    def topics(self) -> list[str]: ...

    @property
    def data(self) -> tuple[list[bytes], bytes]:
        """ :return: A tuple with a list of topics and the log data. """


def fake_exponential(factor: int, numerator: int, denominator: int) -> int: ...
