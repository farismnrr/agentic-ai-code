# Releases

AI Code uses manually reviewed releases. There is no hosted release workflow
and no GitHub Actions release path.

## Product lanes

The repository publishes two products through two independent channels:

| Product | Version metadata | Publication | Release output |
| --- | --- | --- | --- |
| Nuxt web application | Root `package.json` and private TypeScript web-adapter packages | Docker/Buildx to GHCR | OCI image and immutable digest |
| Rust `ai-tools` CLI | `packages/rust-tools/Cargo.toml`, `Cargo.lock`, and native package metadata | GitHub Release | Linux, macOS, and Windows binaries, archives, and checksums |

The lanes may use the same reviewed `main` commit, but their version numbers,
tags, artifacts, and publication commands remain independent. A Nuxt container
publish does not create a CLI release, and a CLI release does not push an OCI
image.

The Nuxt beta line uses `0.1.x-beta`. The native CLI stable line uses its own
`0.0.x` semantic version. For example, the public `v0.1.0-beta` container
release and the CLI `v0.0.14` release are different product releases even when
they are built from related commits. The historical beta GitHub Release must
not be used to infer the CLI's next stable version.

## Version discovery

Before choosing either version, inspect the public history for that product:

```bash
git ls-remote --tags --refs origin 'v0.1.*-beta*'
git ls-remote --tags --refs origin 'v0.0.*'
gh release list --json tagName,isDraft,isPrerelease,publishedAt
gh api --paginate '/users/<owner>/packages/container/ai-code/versions?per_page=100' \
  --jq '.[].metadata.container.tags[]?'
```

Do not infer a version from `latest`, a local build directory, or unrelated
package metadata. Never rewrite a published Git tag or versioned GHCR tag.
When no `0.1.x` beta exists, the first Nuxt beta is `v0.1.0-beta`; later beta
versions increment the highest actually published `0.1.x` beta. The CLI
version increments the highest published native `ai-tools` release and must
match its Cargo package metadata exactly.

## Release branch and tags

Both lanes start from a reviewed release-support PR based on current `main`:

```text
release branch
      -> release-support PR -> main
      -> provider checks/review permitted
      -> product-specific tag(s) on the exact merged main commit
      -> publish the selected product lane
      -> verify remotely and clean the checkout
```

It is valid for `v0.1.1-beta` and `v0.0.14` to point at the same merged commit;
the tags still drive different publishers. Do not publish from a release
branch, feature branch, or stale checkout.

## Nuxt container release

The Nuxt publisher reads the root/web package version and publishes only the
GHCR image. It does not create or edit a GitHub Release. The supported OCI
manifest platforms are:

| OCI platform | Typical host |
| --- | --- |
| `linux/amd64` | x86_64 Linux, Intel macOS Docker Desktop, x86_64 Windows Docker Desktop |
| `linux/arm64` | Apple Silicon macOS Docker Desktop, ARM64 Linux/Windows Docker Desktop |

The image is Linux-based. Docker Desktop on macOS and Windows runs it as a
Linux container; `darwin/*` is not an OCI container platform, and this
Dockerfile does not produce a native Windows-container image. Native macOS and
Windows executables belong to the separate CLI release lane.

From a clean `main` checkout at the intended container tag commit:

```bash
pnpm release:publish:container v0.1.1-beta
```

The publisher uses Buildx with `linux/amd64,linux/arm64`, validates the pushed
manifest, pulls the immutable digest, checks the OCI labels, and verifies both
version tags resolve to that digest. A beta image receives:

```text
ghcr.io/farismnrr/ai-code:v0.1.1-beta
ghcr.io/farismnrr/ai-code:0.1.1-beta
```

The mutable `latest` tag is not moved by a beta publish. A stable Nuxt image
uses `vX.Y.Z`, `X.Y.Z`, and `latest`. Version tags are protected against
silent overwrites; `latest` is the only intentionally mutable alias.

## Rust CLI release targets

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

## Build and publish the CLI

From a clean `main` checkout at the intended CLI tag commit:

```bash
pnpm release:build v0.0.14
```

The CLI build runs the Rust format, clippy, typecheck, test, guardrail, and
audit gates, then performs release-mode builds for all three targets. It
creates `dist/v0.0.14/`:

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
cd dist/v0.0.14
sha256sum --check SHA256SUMS
tar -tzf ai-tools-v0.0.14-x86_64-unknown-linux-gnu.tar.gz
tar -tzf ai-tools-v0.0.14-x86_64-apple-darwin.tar.gz
unzip -t ai-tools-v0.0.14-x86_64-pc-windows-gnu.zip
```

After the CLI tag has been created and pushed from exact `main`:

```bash
pnpm release:publish:cli v0.0.14
```

The CLI publisher fails closed unless the checkout is clean, the current
branch is `main`, the requested tag points exactly at `HEAD` locally and on
`origin`, Cargo metadata matches the tag, GitHub CLI authentication is
available, and the generated bundle has the expected checksums and metadata.
It creates or refreshes a draft GitHub Release, uploads only the native CLI
assets, verifies the bundle again, and then publishes the release. It never
pushes a GHCR image.

`pnpm release:publish` remains a compatibility alias for the CLI publisher;
the explicit `:cli` and `:container` names should be used in new release
notes and operator procedures.

## Container override

Override the image repository only when the destination is intentional:

```bash
GHCR_IMAGE=ghcr.io/<namespace>/ai-code pnpm release:publish:container vX.Y.Z[-prerelease]
```

## Docker and relay boundary

Container publication requires a usable Docker daemon and Buildx builder on
the trusted release host. The MCP coding relay keeps Docker access disabled by
default; run the container publisher from a trusted host shell rather than
weakening the relay or exposing a Docker socket. The release process does not
install, reload, restart, or deploy `ai-tools-relay.service`.

Neither publisher uses GitHub Actions.

## Draft releases

The CLI publisher may leave a GitHub Release draft when asset upload or a
pre-publication check fails; rerun it only after inspecting the failure. GHCR
publication is a separate operation and has no implicit GitHub Release draft.
The container publisher refuses to overwrite existing version tags, while the
stable `latest` alias may move when a new stable Nuxt image is intentionally
published.
