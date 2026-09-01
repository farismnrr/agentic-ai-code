# Releases

AI Code uses manually reviewed releases. There is no hosted release workflow
and no GitHub Actions release path.

## Versioning and discovery

The repository has two deliberately independent product version lines:

- the Nuxt web application uses the `0.1.x` beta line; the root
  `package.json` and the private TypeScript web-adapter packages carry this
  version;
- the native `ai-tools` package uses its own stable semantic version in
  `packages/rust-tools/Cargo.toml`, `Cargo.lock`, and the native package
  metadata. The relay package metadata follows this native version.

Before choosing a release version, inspect all three public surfaces:

```bash
git ls-remote --tags --refs origin 'v0.1.*-beta*'
gh release list --json tagName,isDraft,isPrerelease,publishedAt
gh api --paginate '/users/<owner>/packages/container/ai-code/versions?per_page=100' \
  --jq '.[].metadata.container.tags[]?'
```

Do not infer a version from `latest`, a local build directory, or package
metadata alone. Never rewrite a published tag. When no `0.1.x` beta exists,
the first release in this line follows the repository's existing unnumbered
beta convention: `v0.1.0-beta`. Later beta versions must be selected by
incrementing the highest actually published `0.1.x` beta discovered above.

The GitHub Release tag identifies the web release. The native artifact version
is recorded independently in `RELEASE-METADATA.json` and in each archive name;
the release tooling verifies both values before it builds or publishes.

## Promotion flow

```text
release branch from current main
      -> release PR -> main
      -> provider checks/review permitted
      -> tag v<Nuxt-version> on exact main commit
      -> build/publish GitHub Release + GHCR image
      -> sync and clean main
```

Release work must originate from a reviewed `main` commit. Do not publish from
a release branch, feature branch, or stale checkout.

## Native release targets

The manual bundle currently contains the unified `ai-tools` binary for:

| Target | Package | Runtime note |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `.tar.gz` plus direct binary | Full CLI and Linux/Bubblewrap relay contract |
| `x86_64-apple-darwin` | `.tar.gz` plus direct binary | CLI surfaces; the relay remains Linux-only |
| `x86_64-pc-windows-gnu` | `.zip` plus `.exe` direct binary | CLI surfaces; the relay remains Linux-only |

The macOS and Windows artifacts are real cross-compiled binaries, not renamed
Linux files. The build fails if the Rust target or cross-linker is unavailable.
On the Linux release host, install the reviewed prerequisites once:

```bash
rustup target add x86_64-apple-darwin x86_64-pc-windows-gnu
cargo install cargo-zigbuild --locked
```

`cargo-zigbuild`/Zig is used only when the requested target is not the active
host target. For `x86_64-pc-windows-gnu`, the script explicitly selects
`aws-lc-sys`'s verified prebuilt x86_64 NASM objects so a Linux release host does
not need a host NASM installation. The script validates ELF, Mach-O, or PE
architecture metadata and checks every archive before publishing.

## Build release artifacts

From a clean `main` checkout at the intended tag commit:

```bash
pnpm release:build v0.1.0-beta
```

The release build runs the web and native lint/type/test gates, the guardrail,
dependency audits, the Nuxt production build, and release-mode native builds.
It creates `dist/v0.1.0-beta/` with the native version resolved from Cargo:

```text
ai-tools-x86_64-unknown-linux-gnu
ai-tools-v0.0.14-x86_64-unknown-linux-gnu.tar.gz
ai-tools-x86_64-apple-darwin
ai-tools-v0.0.14-x86_64-apple-darwin.tar.gz
ai-tools-x86_64-pc-windows-gnu.exe
ai-tools-v0.0.14-x86_64-pc-windows-gnu.zip
RELEASE-METADATA.json
SHA256SUMS
```

Validate a copied bundle with:

```bash
cd dist/v0.1.0-beta
sha256sum --check SHA256SUMS
tar -tzf ai-tools-v0.0.14-x86_64-unknown-linux-gnu.tar.gz
tar -tzf ai-tools-v0.0.14-x86_64-apple-darwin.tar.gz
unzip -t ai-tools-v0.0.14-x86_64-pc-windows-gnu.zip
```

## Publish

After the tag has been created and pushed from exact `main`:

```bash
pnpm release:publish v0.1.0-beta
```

The publish script fails closed unless:

- the checkout is clean and the current branch is `main`;
- the requested tag points exactly at `HEAD` locally and on `origin`;
- the web and native metadata agree with the release bundle;
- GitHub CLI authentication and a Docker Buildx daemon are available.

It builds and verifies all native assets, creates or refreshes a draft GitHub
Release, pushes the `linux/amd64` web image, pulls and inspects the immutable
image digest, checks both version tags resolve to that digest, and only then
publishes the GitHub Release. If the OCI push fails, the GitHub Release stays
draft for a safe retry.

For a beta web release, the image tags are:

```text
ghcr.io/farismnrr/ai-code:v0.1.0-beta
ghcr.io/farismnrr/ai-code:0.1.0-beta
```

The mutable `latest` tag remains on the last stable image and is not moved by
a beta publish. Stable releases continue to use `vX.Y.Z`, `X.Y.Z`, and
`latest`, with the web image intentionally **linux/amd64-only**. ARM64
publication was removed after the emulated Nuxt/Vite build caused severe host
swap thrashing; do not add it without a reviewed native/remote ARM64 builder.

Override the image repository only when the destination is intentional:

```bash
GHCR_IMAGE=ghcr.io/<namespace>/ai-code pnpm release:publish vX.Y.Z
```

## Docker and relay boundary

Web image publication requires a usable Docker daemon/build environment on the
host running `release:publish`. The MCP coding relay keeps Docker access
disabled by default; run this publisher from a trusted host shell rather than
weakening the relay or exposing a Docker socket. The release process does not
install, reload, restart, or deploy `ai-tools-relay.service`.

## Draft releases

It is valid for the GitHub Release and native assets to remain draft while the
web image is unavailable. The publisher intentionally keeps the release draft
until registry publication and immutable-image verification succeed.
