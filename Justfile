#!/usr/bin/env -S just --justfile

set dotenv-load := true
set positional-arguments := true

default:
    just --list --unsorted

alias a := allow
allow:
    direnv allow

shell:
    devenv shell

sync:
    uv sync --all-groups

fmt:
    cargo fmt
    uv run ruff format python

fmt-check:
    cargo fmt --check
    uv run ruff format --check python

lint:
    cargo clippy --all-targets --all-features -- -D warnings
    uv run ruff check python

test:
    cargo test --all-features
    uv run pytest

check: fmt-check lint test

build:
    cargo build --release
    uv run maturin build --release

develop:
    uv run maturin develop
