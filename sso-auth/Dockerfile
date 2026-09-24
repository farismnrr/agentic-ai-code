FROM node:22-alpine AS frontend-builder

WORKDIR /build

COPY sso-auth/package.json ./
RUN npm install

COPY sso-auth/index.html ./
COPY sso-auth/tsconfig.json ./
COPY sso-auth/vite.config.ts ./
COPY sso-auth/frontend ./frontend

RUN npm run build


FROM rust:1-alpine AS backend-builder

RUN apk add --no-cache musl-dev

WORKDIR /build

COPY sso-auth/Cargo.toml ./
COPY sso-auth/src ./src
COPY --from=frontend-builder /build/dist ./dist

RUN cargo build --release


FROM alpine:3.22 AS certificates
RUN apk add --no-cache ca-certificates


FROM scratch

COPY --from=certificates /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=backend-builder /build/target/release/sso-auth /sso-auth

ENV PORT=3000
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

EXPOSE 3000

USER 65532:65532

ENTRYPOINT ["/sso-auth"]
