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

```python
from pyrevm import EVM

evm = EVM();
# TODO
```


## Develop

We use Poetry for virtual environment management and [Maturin](https://github.com/PyO3/maturin) as our Rust <> Python FFI build system. The Rust bindings are auto-generated from the macros provided by [PyO3](https://pyo3.rs/v0.17.1/).

To build the library, run `poetry run maturin develop`

Note: If building for production, do not forget the `--release` flag, else performance will be degraded.

## Benchmarks

TODO
