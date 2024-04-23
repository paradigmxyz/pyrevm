# <h1 align="center"> pyrevm </h1>

**Blazing-fast Python bindings to [revm](https://github.com/bluealloy/revm/)**

![py](https://github.com/gakonst/pyrevm/workflows/py/badge.svg)
![rust](https://github.com/gakonst/pyrevm/workflows/rust/badge.svg)

## Quickstart

```
make install
make test
```

## Example Usage

Here we show how you can fork from Ethereum mainnet and simulate
a transaction from `vitalik.eth`.

```python
from pyrevm import EVM, Env, BlockEnv

address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"  # vitalik.eth
address2 = "0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"

fork_url = "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27"

# set up an evm
evm = EVM(
    # can fork from a remote node
    fork_url=fork_url,
    # can set tracing to true/false
    tracing=True,
    # can configure the environment
    env=Env(
        block=BlockEnv(timestamp=100)
    )
)

vb_before = evm.basic(address)
assert vb_before != 0

# Execute the tx
evm.message_call(
    caller=address,
    to=address2,
    value=10000
    # data
)

assert vb_before != evm.basic(address)
assert evm.basic(address2).balance == 10000
```

### Tracing
There is also support for tracing:
```python
from pyrevm import EVM

EVM(tracing=True)
```

### Transactions

There is support for checkpoints:

```python
from pyrevm import EVM

evm = EVM()
checkpoint = evm.snapshot()
evm.message_call(
    caller=...,
    to=...,
    value=...,
)
evm.revert(checkpoint)  # or: evm.commit() to clear all checkpoints
```

**Note**: in contrast to the Rust library, the Python library does not automatically commit to database.

See more usage examples in the [pytests](tests/test_evm.py).

## Develop

We use Poetry for virtual environment management and [Maturin](https://github.com/PyO3/maturin) as our Rust <> Python FFI build system. The Rust bindings are auto-generated from the macros provided by [PyO3](https://pyo3.rs/v0.17.1/).

To build the library, run `make build`. To run the tests, run `make test`.

Note: If building for production, use `make build-prod`, else performance will be degraded.

To release to pypi, create a new github release. This will run the `.github/workflows/release.yml` action and publish source+binary wheels to pypi.

## Benchmarks

TODO
