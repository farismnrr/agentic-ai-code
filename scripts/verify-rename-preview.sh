#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo run -q -p relay-application --example rename_preview_acceptance --locked
