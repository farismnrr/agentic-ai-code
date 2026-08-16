# Releases

AI Code uses manual reviewed releases. There is no GitHub Actions release workflow.

## Versioning

The stable line currently uses `0.0.x` semantic versions. The first stable line after the existing prerelease history is `0.0.8`; do not reset versioning to `0.0.1` because earlier beta tags already established higher versions.

Keep the root web/package version, Cargo workspace/CLI version, stable Git tag, and release assets aligned.

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

It then builds/verifies assets, uploads GitHub Release assets, publishes the multi-architecture web image to GHCR, and only after those steps publishes the GitHub Release.

Stable web image tags are:

```text
vX.Y.Z
X.Y.Z
latest
```

Default image repository:

```text
ghcr.io/farismnrr/ai-code
```

## Docker requirement

Web image publication requires a usable Docker daemon/build environment on the host running `release:publish`.

The MCP coding relay intentionally has no Docker socket. Therefore invoking the release publisher from inside the restricted relay sandbox may successfully run source/build checks but must fail at the Docker step. Do not weaken the relay by exposing `/var/run/docker.sock`; run the release publisher from a normal trusted host shell with Docker available.

## Draft releases

It is valid for the GitHub Release and CLI assets to exist as a draft while the web image is not yet published. The atomic publisher intentionally keeps the GitHub Release draft until GHCR publication succeeds, preventing a half-published stable release.
