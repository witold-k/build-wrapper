# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Witold Kaminski

target_dir := `cargo metadata --no-deps --format-version=1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])'`

default: build

pull:
    python3 build.py github_pull

build:
    python3 build.py build-debug

build-release:
    python3 build.py build-release

gbuild: pull build

rbuild: pull build-release install

test:
    python3 build.py test

check:
    python3 build.py build-debug

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --all-targets -- -D warnings

install:
    cargo build --release
    mkdir -p "$HOME/bin"
    cp "{{target_dir}}/release/build-wrapper" "$HOME/bin/build-wrapper"

run *args:
    cargo run -- {{args}}

cover-setup:
    cargo install cargo-llvm-cov
    rustup component add llvm-tools-preview

cover:
    cargo llvm-cov --all-targets --html

cover-lcov:
    cargo llvm-cov --all-targets --lcov --output-path "{{target_dir}}/coverage/lcov.info"

cover-text:
    cargo llvm-cov --all-targets

clean:
    cargo clean -p build-wrapper

clean-all:
    cargo clean
