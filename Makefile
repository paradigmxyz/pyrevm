.PHONY: default install build run

default: test

install:
	poetry install

build:
	poetry run maturin develop

build-prod:
	poetry run maturin develop --release

test: build
	poetry run pytest -s tests/*

lint:
	cargo clippy --workspace --all-targets --all-features
	cargo fmt --all
