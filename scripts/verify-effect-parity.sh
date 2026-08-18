#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
node --experimental-strip-types scripts/verify-effect-parity.ts
expected=$(node --input-type=module -e "import fs from 'node:fs'; process.stdout.write(JSON.stringify(JSON.parse(fs.readFileSync('.agents/contracts/039e-effect-classification.json', 'utf8'))))")
actual=$(cargo run --quiet --locked -p relay-application --example effect_parity_acceptance)
RUST_EFFECTS="$actual" EXPECTED_EFFECTS="$expected" node --input-type=module -e "const rust=JSON.parse(process.env.RUST_EFFECTS); const expected=JSON.parse(process.env.EXPECTED_EFFECTS); if (rust.length !== expected.length || rust.some((item, i) => item.tool !== expected[i].tool || JSON.stringify(item.effects) !== JSON.stringify(expected[i].effects))) process.exit(1)" || { echo 'effect classification parity mismatch between Rust and first-party contract' >&2; exit 1; }
echo 'effect classification parity: pass'
