#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cargo run --locked --quiet --manifest-path "$root/packages/rust-tools/application/Cargo.toml" --example hooks_acceptance
