#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo run -q -p relay-application --example post_edit_diagnostics_acceptance --locked
