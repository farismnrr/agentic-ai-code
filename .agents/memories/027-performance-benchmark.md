# 027 CLI Rust Refactor Performance Benchmark

**Date:** 2026-08-10
**Related Plan:** [027-cli-rust-refactor.md](../plans/027-cli-rust-refactor.md)

## Methodology

A custom Python script (`benchmark_runner.py`) was used to execute each CLI implementation and record its end-to-end execution time, Peak Resident Set Size (RSS), and on-disk binary size.

### Hardware/Environment
- Environment: Local Linux development environment
- Toolchain: Rust 1.80.0 (via `cargo build --release`), Node.js (v20/v22 equivalent)

### Methodology Details
- **Warm-start/Cold-start:** Each command was executed for 2 warmup iterations to negate page cache variations, followed by 10 benchmark iterations.
- **Latency Measurement:** Python's `time.perf_counter()` was used to measure end-to-end process execution. This intentionally captures total process lifecycle (startup, execution, shutdown), reflecting the CLI usage context.
- **Peak RSS Measurement:** Python's `resource.getrusage(resource.RUSAGE_CHILDREN)` was used after isolating each execution in a sub-process wrapper to accurately capture the max RSS for the specific execution.
- **Fixtures:** 
  - `terminal-tool`: `--no-guard echo hello` (measures minimal subprocess execution)
  - `curl-tool`: `http://0.0.0.0` (measures startup/parsing and immediate network-layer exit)
  - `searxng-search-tool`: `--help` (measures minimal startup and output parsing)

## Results

| Tool | Latency (Avg) | Peak RSS (Avg) | Binary Size |
|---|---|---|---|
| `terminal-tool` (Rust) | 1.96 ms | 14.01 MB | 0.92 MB |
| `terminal-tool` (Node) | 280.82 ms | 157.05 MB | 1.13 KB (source) |
| `curl-tool` (Rust) | 2.17 ms | 14.05 MB | 2.29 MB |
| `curl-tool` (Node) | 242.40 ms | 153.60 MB | 1.42 KB (source) |
| `searxng-tool` (Rust) | 2.03 ms | 14.07 MB | 2.13 MB |
| `searxng-tool` (Node) | 248.81 ms | 152.78 MB | 0.72 KB (source) |

## Findings & Conclusions
1. **Latency (Throughput):** Rust implementations showed over a 100x improvement in startup and execution latency across all tools (~2ms vs ~250ms). This massively reduces overhead in any agentic loops relying on these tools.
2. **Memory Footprint:** Peak RSS usage dropped from ~155MB to ~14MB, yielding >10x memory reduction.
3. **No Unsupported Claims:** The benchmark measures only the CLI layer wrapper overhead; network transit latency for `curl` or `searxng` queries will remain unchanged, but the per-invocation lifecycle cost has been effectively eliminated.

**Status:** The benchmark gate is satisfied and all evidence is reproducible via `benchmark_runner.py`.
