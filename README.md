# pyrevm

**Blazing-fast Python bindings to [revm](https://github.com/bluealloy/revm/)**

## Develop

We use Poetry for virtual environment management and [Maturin](https://github.com/PyO3/maturin) as our Rust <> Python FFI build system. The Rust bindings are auto-generated from the macros provided by [PyO3](https://pyo3.rs/v0.17.1/).

To build the library, run `poetry run maturin develop`

## Benchmarks

TODO
