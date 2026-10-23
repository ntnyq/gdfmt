#!/usr/bin/env bash
set -euo pipefail

# The playground ships the workspace's Rust formatter, compiled to WebAssembly.
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error https://sh.rustup.rs |
    sh -s -- -y --profile minimal --default-toolchain none
fi
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

# rustup reads the repository's rust-toolchain.toml from the docs directory.
rustup target add wasm32-unknown-unknown
if ! command -v wasm-pack >/dev/null 2>&1; then
  cargo install wasm-pack --version 0.15.0 --locked
fi
# Run each build from Bash: the build image's pnpm launcher cannot be spawned
# by pnpm's shell emulator when a package script invokes pnpm again.
pnpm --dir .. exec tsx scripts/wasm/build.ts
pnpm exec vitepress build .
