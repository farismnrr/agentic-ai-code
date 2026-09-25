# syntax=docker/dockerfile:1.7

FROM rust:1-bookworm AS base

RUN rustup component add rustfmt clippy

WORKDIR /build

COPY relay-agent/Cargo.toml ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

FROM base AS check

COPY relay-agent/src ./src
COPY relay-agent/tests ./tests

RUN cargo fmt --check
RUN cargo clippy --all-targets --all-features -- -D warnings
RUN cargo test --all-targets --all-features

FROM base AS builder

COPY relay-agent/src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

COPY --from=builder /build/target/release/relay-agent /relay-agent

EXPOSE 3100
ENTRYPOINT ["/relay-agent"]
