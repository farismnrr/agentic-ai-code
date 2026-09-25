# syntax=docker/dockerfile:1.7

FROM node:22-bookworm-slim AS frontend-deps

WORKDIR /build

COPY sso-auth/package.json ./

RUN --mount=type=cache,target=/root/.npm \
    npm install --legacy-peer-deps --prefer-offline --no-audit --no-fund


FROM frontend-deps AS frontend-builder

ARG VITE_AGENTATION_ENABLED=false
ENV VITE_AGENTATION_ENABLED=${VITE_AGENTATION_ENABLED}

COPY sso-auth/index.html ./
COPY sso-auth/tsconfig.json ./
COPY sso-auth/vite.config.ts ./
COPY sso-auth/public ./public
COPY sso-auth/frontend ./frontend

RUN npm run build


FROM rust:1-bookworm AS chef

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install cargo-chef --locked

WORKDIR /build


FROM chef AS planner

COPY sso-auth/Cargo.toml ./
COPY sso-auth/src ./src

RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS backend-builder

COPY --from=planner /build/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

COPY sso-auth/Cargo.toml ./
COPY sso-auth/src ./src
COPY --from=frontend-builder /build/dist ./dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release


FROM gcr.io/distroless/cc-debian12:nonroot

LABEL org.opencontainers.image.source="https://github.com/farismnrr/agentic-ai-code"

COPY --from=backend-builder /build/target/release/sso-auth /sso-auth

ENV PORT=3000

EXPOSE 3000

ENTRYPOINT ["/sso-auth"]
