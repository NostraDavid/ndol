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

setup:
    uv sync --locked --all-groups

update:
    devenv update
    cargo update
    uv lock --upgrade
    uv sync --locked --all-groups

sync:
    uv sync --all-groups

fmt:
    cargo fmt
    uv run ruff format python

fmt-check:
    cargo fmt --check
    uv run ruff format --check python

lint:
    cargo clippy --locked --all-targets --all-features -- -D warnings
    uv run ruff check --no-fix python

test:
    cargo test --locked --all-features
    uv run pytest

check: lock-check fmt-check lint test

lock-check:
    cargo metadata --locked --format-version 1 --no-deps > /dev/null
    uv lock --check

build:
    cargo build --locked --release
    uv run maturin build --locked --release

develop:
    uv run maturin develop
