#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo run -q -p relay-application --example typescript_lsp_acceptance --locked
