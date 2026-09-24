FROM node:22-bookworm-slim AS frontend-builder

WORKDIR /build

COPY sso-auth/package.json ./
RUN npm install

COPY sso-auth/index.html ./
COPY sso-auth/tsconfig.json ./
COPY sso-auth/vite.config.ts ./
COPY sso-auth/frontend ./frontend

RUN npm run build


FROM rust:1-bookworm AS backend-builder

WORKDIR /build

COPY sso-auth/Cargo.toml ./
COPY sso-auth/src ./src
COPY --from=frontend-builder /build/dist ./dist

RUN cargo build --release


FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=backend-builder /build/target/release/sso-auth /sso-auth

ENV PORT=3000

EXPOSE 3000

ENTRYPOINT ["/sso-auth"]
