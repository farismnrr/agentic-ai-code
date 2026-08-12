# 027 CLI Rust Refactor Performance Benchmark

**Date:** 2026-08-10  
**Related Plan:** [027-cli-rust-refactor.md](../plans/027-cli-rust-refactor.md)

## Status

This file preserves **historical migration evidence** from the JS → Rust CLI cutover. The original one-off Python benchmark runner and JavaScript oracle CLIs were removed after the zero-JS cutover, so these exact comparative measurements are **not a current runnable benchmark suite** and must not be described as such.

If a new performance claim is needed, create a current benchmark against the current Rust binaries and record fresh environment/toolchain details rather than reconstructing or reintroducing deleted JavaScript CLI launchers.

## Historical methodology

A custom Python runner executed each then-current CLI implementation and recorded end-to-end execution time, Peak Resident Set Size (RSS), and on-disk binary/source size.

### Hardware/environment

- Environment: local Linux development environment (Ubuntu 22.04)
- CPU: AMD Ryzen 9 5900X (or equivalent)
- Toolchain: Rust 1.95.0 (`cargo build --release`) and Node.js 22

### Methodology details

- **Warmup/sample count:** 2 warmup iterations followed by 10 measured iterations.
- **Latency:** Python `time.perf_counter()` measured total process lifecycle (startup, execution, shutdown).
- **Peak RSS:** Python `resource.getrusage(resource.RUSAGE_CHILDREN)` after isolating each execution in a subprocess wrapper.
- **Historical fixtures:**
  - `terminal-tool`: `--no-guard echo hello`
  - `curl-tool`: `http://0.0.0.0`
  - `searxng-search-tool`: `--help`

## Recorded results

| Tool | Latency (Avg) | Peak RSS (Avg) | Binary Size |
|---|---|---|---|
| `terminal-tool` (Rust) | 1.96 ms | 14.01 MB | 0.92 MB |
| `terminal-tool` (Node) | 280.82 ms | 157.05 MB | 1.13 KB (source) |
| `curl-tool` (Rust) | 2.17 ms | 14.05 MB | 2.29 MB |
| `curl-tool` (Node) | 242.40 ms | 153.60 MB | 1.42 KB (source) |
| `searxng-tool` (Rust) | 2.03 ms | 14.07 MB | 2.13 MB |
| `searxng-tool` (Node) | 248.81 ms | 152.78 MB | 0.72 KB (source) |

## Historical conclusion

The recorded migration run showed materially lower process-startup latency and memory use for the Rust CLI layer. The measurement covered CLI process lifecycle overhead only; it did not claim network transit improvements for curl/SearXNG operations.

Treat these numbers as evidence supporting the completed Plan 027 migration decision, **not as a continuously reproducible current benchmark or a guarantee of present-day performance**.
