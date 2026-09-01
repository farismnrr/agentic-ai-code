# Releases

AI Code uses manually reviewed releases. There is no hosted release workflow.

## Versioning

The stable line uses `0.0.x` semantic versions. Continue from the current
stable tag recorded by the repository rather than rewriting published tags or
resetting versioning.

Keep the root web/package version, `ai-tools` Cargo package version, stable Git tag, and release assets aligned.

## Promotion flow

```text
implementation branch
      -> PR -> dev
      -> release PR -> main
      -> stable tag on exact main commit
      -> build/publish release
```

Release work must originate from a reviewed `main` commit. Do not publish from `dev` or a feature branch.

## Build release artifacts

From a clean `main` checkout at the intended tag commit:

```bash
pnpm release:build vX.Y.Z
```

The release build runs the mandatory local gate and production builds, then creates `dist/vX.Y.Z/` output including:

```text
ai-tools-x86_64-unknown-linux-gnu
ai-tools-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
SHA256SUMS
```

The production Rust relay contract remains Linux + Bubblewrap.

## Publish

```bash
pnpm release:publish vX.Y.Z
```

The publish script fails closed unless:

- the checkout is clean;
- current branch is `main`;
- the requested stable tag points exactly at `HEAD`;
- that tag is already present on `origin`.

It then builds/verifies assets, uploads release assets to the configured forge,
publishes the `linux/amd64` web image to the configured OCI registry, and only
after those steps publishes the release record.

Stable web image tags are:

```text
vX.Y.Z
X.Y.Z
latest
```

The stable web image is intentionally **linux/amd64-only**. ARM64 publication was removed after the emulated Nuxt/Vite build hit severe host swap thrashing under QEMU; do not re-add `linux/arm64` without a native/remote ARM64 builder and a fresh release-path review.

The image repository is configured by the release environment:

```text
<oci-registry>/<namespace>/ai-code
```

## Docker requirement

Web image publication requires a usable Docker daemon/build environment on the host running `release:publish`.

The MCP coding relay keeps Docker access disabled by default. A trusted single-owner local coding relay may explicitly opt in with `--allow-docker` / `RELAY_ALLOW_DOCKER=true`, which exposes only the configured Docker daemon socket to the Bubblewrap sandbox. Docker daemon access is effectively host-level authority, so production/remote relays should keep it disabled unless the operator deliberately accepts that trust expansion. When release publication is run through MCP, verify Docker client/server access first and ensure the relay was intentionally started with Docker enabled.

## Draft releases

It is valid for the release record and CLI assets to exist as a draft while
the web image is not yet published. The atomic publisher intentionally keeps
the release draft until registry publication succeeds, preventing a
half-published stable release.
