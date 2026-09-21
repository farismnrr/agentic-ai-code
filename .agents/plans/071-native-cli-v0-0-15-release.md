# Plan 071 — Native CLI v0.0.15 Release

Created: 2026-09-20
Updated: 2026-09-20

Status: **IN PROGRESS — release support source is being prepared on `release/v0.0.15`. Publication must occur only from the exact reviewed `main` merge commit after local cross-target build verification.**

## Goal

Publish native `ai-tools v0.0.15` as the next patch release without changing the independent Nuxt/web version lane.

The GitHub Release must contain:

- `x86_64-unknown-linux-gnu` direct binary + tarball;
- `aarch64-unknown-linux-gnu` direct binary + tarball;
- `x86_64-apple-darwin` direct binary + tarball;
- `aarch64-apple-darwin` direct binary + tarball;
- `x86_64-pc-windows-gnu` direct `.exe` + ZIP;
- versioned `masih-awam-workspace-workflow` Skill ZIP generated from the tracked canonical Skill;
- `RELEASE-METADATA.json`;
- `SHA256SUMS`.

The relay subcommand remains Linux/Bubblewrap-only. macOS and Windows artifacts are portable CLI binaries and must not be documented as non-Linux relay support.

## Delivery sequence

1. Implement and verify release-support changes on `release/v0.0.15`.
2. Run the five-target release build and validate every archive/checksum/metadata entry.
3. Commit, push, open a release-support PR into `main`, merge it, return to clean `main`, and delete the release branch.
4. Create and push `v0.0.15` on the exact merged `main` commit.
5. Publish the CLI GitHub Release from clean `main`.
6. Verify the published release assets and checksums remotely.
7. Finish with only local/remote `main`; no stale release branch.

## Acceptance

- [x] Highest published native tag is `v0.0.14`; next patch is `v0.0.15`.
- [x] Native manifests and Cargo lock use `0.0.15`.
- [x] Release bundle source includes Linux amd64/arm64, macOS x86_64/arm64, and Windows x86_64.
- [x] Release build generates the Skill ZIP from `.agents/skills/masih-awam-workspace-workflow/SKILL.md` and includes it in metadata/checksums.
- [x] Publisher derives the expected asset count from release metadata and requires the Skill archive.
- [x] Release script syntax, locked host Cargo check, Skill ZIP layout smoke, and diff hygiene pass.
- [x] Base release-host cross-build prerequisites are installed without weakening repository/runtime policy: the four additional Rust targets are installed, Zig 0.16.0 is available, and `cargo-zigbuild` 0.23.4 is installed in the operator user's Cargo bin directory. `cargo zigbuild --version` is not a supported subcommand flag; installation output plus executable/help discovery are the verification path.
- [x] Portable non-Linux source drift found by the first macOS cross-build was fixed without warning suppression: the portable freshness guard now matches the Linux lifetime contract, Linux-only `SpawnControl::remaining` is cfg-gated, and unused portable helpers were removed.
- [x] Non-macOS release hosts now require a real `SDKROOT` for Apple targets so the supported cargo-zigbuild path is explicit and the unavailable-`xcrun` warning is not accepted as normal release output.
- [x] A macOS SDK is installed/provided on the Linux release host and exported through `SDKROOT`; the subsequent full cross-build completed both Intel and Apple Silicon targets without the unavailable-`xcrun` warning.
- [x] A pre-final `pnpm release:build v0.0.15` completed all five native targets, Skill ZIP packaging, metadata, and checksums; it exposed Windows-only warning debt that is being fixed before release publication.
- [x] Native release target compilation now denies Rust warnings (`-D warnings`) so platform-specific cfg drift cannot be accepted as warning-only release output.
- [x] Final post-warning-fix `pnpm release:build v0.0.15` passes for all five native targets with zero Rust warnings; Skill ZIP, metadata, archives, and SHA256 verification also pass.
- [x] Release-support branch fast/full applicable gates pass as part of the final release build; release target compilation also runs with `-D warnings`.
- [ ] Release-support PR is pushed, reviewed/checked, and merged to `main`.
- [ ] `v0.0.15` is created/pushed at the exact merged `main` commit.
- [ ] `pnpm release:publish:cli v0.0.15` publishes the GitHub Release.
- [ ] Published assets/checksums/metadata are remotely verified.
- [ ] Release branch is deleted locally/remotely and final checkout is clean `main` only.
