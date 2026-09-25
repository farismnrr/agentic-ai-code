# syntax=docker/dockerfile:1.7

ARG BUILDPLATFORM=linux/amd64

FROM --platform=${BUILDPLATFORM} node:22-bookworm-slim AS frontend-deps

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
COPY sso-auth/assets ./assets
COPY sso-auth/frontend ./frontend

RUN node -e "const fs=require('fs'); const dir='assets/security-auth'; const data=fs.readdirSync(dir).sort().map(name=>fs.readFileSync(dir+'/'+name,'utf8')).join(''); fs.writeFileSync('public/assets/security-auth.png',Buffer.from(data,'base64'))"

RUN npm run build


FROM --platform=${BUILDPLATFORM} rust:1-bookworm AS chef-base

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install cargo-chef --locked

WORKDIR /build


FROM chef-base AS planner

COPY sso-auth/Cargo.toml ./
COPY sso-auth/src ./src

RUN cargo chef prepare --recipe-path recipe.json


FROM chef-base AS backend-builder

ARG TARGETARCH=amd64

ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

RUN case "${TARGETARCH}" in \
      amd64) target="x86_64-unknown-linux-gnu" ;; \
      arm64) target="aarch64-unknown-linux-gnu" ;; \
      *) echo "unsupported target architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && printf '%s' "${target}" > /tmp/rust-target \
    && if [ "${TARGETARCH}" = "arm64" ]; then \
         apt-get update \
         && apt-get install -y --no-install-recommends gcc-aarch64-linux-gnu \
         && rm -rf /var/lib/apt/lists/*; \
       fi \
    && rustup target add "${target}"

COPY --from=planner /build/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    target="$(cat /tmp/rust-target)" \
    && cargo chef cook --release --target "${target}" --recipe-path recipe.json

COPY sso-auth/Cargo.toml ./
COPY sso-auth/src ./src
COPY --from=frontend-builder /build/dist ./dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    target="$(cat /tmp/rust-target)" \
    && cargo build --release --target "${target}" \
    && cp "target/${target}/release/sso-auth" /build/sso-auth


FROM gcr.io/distroless/cc-debian12:nonroot

LABEL org.opencontainers.image.source="https://github.com/farismnrr/agentic-ai-code"

COPY --from=backend-builder /build/sso-auth /sso-auth

ENV PORT=3000

EXPOSE 3000

ENTRYPOINT ["/sso-auth"]
