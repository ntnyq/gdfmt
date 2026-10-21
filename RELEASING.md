# Publishing gdfmt

[简体中文](RELEASING.zh-CN.md)

The Rust workspace and public npm packages share one version. A `vVERSION` tag runs the full Rust CI and the complete npm matrix, freezes the artifacts, publishes crates.io, publishes npm, and then creates a GitHub Release. Ordinary branch pushes and pull requests only validate changes.

Registry writes are disabled until the GitHub repository variable `RELEASE_ENABLED` is exactly `true`. Manual workflow runs default to validation only. No registry credentials are needed to build, test or freeze artifacts.

## Release units

Rust packages publish in this dependency order:

```text
gdfmt_text → gdfmt_syntax → gdfmt_parser → gdfmt_formatter
→ gdfmt → gdfmt_config → gdfmt_workspace → gdfmt_cli
```

`gdfmt_napi` and `xtask` remain unpublished. The CLI crate is `gdfmt_cli`; its installed executable is `gdfmt`:

```sh
cargo install gdfmt_cli --version '=VERSION' --locked
```

The npm release contains `gdfmt` and these optional native packages:

```text
@gdfmt/darwin-arm64
@gdfmt/darwin-x64
@gdfmt/linux-x64-gnu
@gdfmt/linux-x64-musl
@gdfmt/win32-x64-msvc
```

All five platform versions must match the root package exactly. Prereleases use npm's `next` channel; stable versions use `latest`. Each package is first uploaded under `candidate`. The release channels are advanced only after all six versions are visible with matching integrity and an isolated registry consumer passes the Node API and CLI smoke checks. Platform channels advance before the root channel. The `candidate` tag is retained for inspection.

## One-time platform setup

### GitHub

In `ntnyq/gdfmt`:

1. Enable GitHub Actions and allow the actions used by the workflows. Public repositories should use GitHub-hosted runners, including the macOS Intel and arm64 runners used by the native matrix.
2. Create an environment named **`release`**. Allow version tags matching `v*`. Leave required reviewers unset for automatic publication, or configure them if publication should require approval.
3. Protect the default branch and use a tag ruleset to restrict creation, modification and deletion of `v*` tags to release maintainers. Release preflight also requires the tagged commit to be reachable from the default branch.
4. Leave **Settings → Secrets and variables → Actions → Variables → `RELEASE_ENABLED`** unset until bootstrap and Trusted Publisher setup are complete. Set it to **`true`** when ready.

No `NPM_TOKEN`, `CARGO_REGISTRY_TOKEN` or personal GitHub token secret is needed for subsequent releases. The crates.io job exchanges OIDC for a temporary token; npm uses OIDC directly. Only the final GitHub Release job receives `contents: write`.

### crates.io

1. Sign in, verify your email and verify ownership or availability of all eight crate names.
2. Create a limited API token for the first publication and authenticate locally with `cargo login`. New crate creation needs the corresponding token permission.
3. Complete the first real publication as described below.
4. In **each of the eight crates**, configure a GitHub Actions Trusted Publisher with these values:

| Field             | Value         |
| ----------------- | ------------- |
| Repository owner  | `ntnyq`       |
| Repository        | `gdfmt`       |
| Workflow filename | `release.yml` |
| Environment       | `release`     |

After OIDC has published successfully, revoke the bootstrap token and optionally enable Trusted Publishing Only mode. Do not configure `npm.yml` or `ci.yml` as the publishing workflow: publication happens directly in `release.yml`.

### npm

1. Verify your account/email and enable two-factor authentication.
2. Create or obtain publishing access to the **`@gdfmt` scope** and verify availability or ownership of the unscoped **`gdfmt`** name. GitHub repository ownership does not grant npm scope ownership.
3. Ensure the publishing account can create and update all six public packages. Complete bootstrap below using interactive authentication.
4. In **each of the six packages**, add a GitHub Actions Trusted Publisher:

| Field                | Value         |
| -------------------- | ------------- |
| Organization or user | `ntnyq`       |
| Repository           | `gdfmt`       |
| Workflow filename    | `release.yml` |
| Environment          | `release`     |
| Allow `npm publish`  | Enabled       |
| Allow `npm dist-tag` | Enabled       |

Enter only the workflow filename, including `.yml`. New configurations can default to staged publishing; this workflow needs direct publication and separate dist-tag permission. A newly configured npm publisher must successfully publish a **new version within two days**. Skipping an already-published version during a retry does not validate that new trust configuration. If it expires, recreate it before the next release.

The publishing job pins Node 24.0.0 and npm 11.21.0, which supports OIDC publication and dist-tag management. These are publishing requirements; the consumer matrix still verifies Node 22.13.0 and 24.0.0. Once OIDC works, npm package settings can require 2FA and disallow token publication.

## Bootstrap the first release

Trusted Publisher configuration requires existing packages. Bootstrap is a separate, intentional publication by a maintainer. Do not enable `RELEASE_ENABLED` yet, and do not publish empty placeholder packages.

1. Merge the release configuration and the desired initial version into the default branch. Commit all release changes. Use the repository's pinned Rust toolchain and Node 24.0.0 for release preparation.
2. Create the matching tag and push it. With publishing disabled, this runs validation and produces frozen artifacts without uploading any packages.
3. Wait for the **Release** workflow's `freeze` job to succeed. Record the run ID and the exact artifact name from that successful freeze attempt, then download the single bundle:

```sh
gh run download RUN_ID --name release-artifacts-FREEZE_ATTEMPT --dir target/release
```

4. Use a clean checkout of that exact tag. Check the version and inspect the read-only registry plans:

```sh
pnpm install --frozen-lockfile
pnpm release:check --tag v0.1.0-alpha.0
pnpm release:publish:crates
pnpm release:publish:npm
```

Replace the example tag with the actual version. Replace `FREEZE_ATTEMPT` with the successful freeze attempt (for example, `1`); a later publisher-only retry does not change that artifact name. The bundle contains `npm/` and `crates/`. Downloaded `npm/release.json` records the commit, clean-source status, complete matrix, selected files and SHA-512 integrity. `crates/crates.json` binds all eight Rust archives to the same commit and SHA-256 checksums, and the verifier also checks each archive's Cargo VCS metadata. The selected native tarballs come from the Node 24.0.0 runs; the Node 22.13.0 runs independently test the same root tarball. Native artifacts built by the two Node jobs are not required to be byte-identical.

5. When ready to perform the first actual publication, use npm 11.21+ or 12.2+, authenticate and explicitly execute the plans. Node 24.0.0's originally bundled npm is older, so upgrade it first:

```sh
npm install --global npm@11.21.0 --ignore-scripts
cargo login
npm login
pnpm release:publish:crates --execute
pnpm release:publish:npm --execute
```

The two `--execute` commands perform real registry writes. They require a clean checkout, including staged and untracked files. The npm publisher invokes npm's JavaScript CLI with the active Node runtime instead of executing `npm.cmd`; for a custom npm installation prefix, set `GDFMT_NPM_CLI` to that installation's `npm/bin/npm-cli.js`. The scripts publish missing packages in order, verify registry checksums, and stop on conflicts. crates.io packaging must resolve against crates.io; registry source replacement can interfere with Cargo's temporary workspace registry. Use a clean release environment if local Cargo configuration changes registry resolution.

6. Configure the Trusted Publishers above and enable `RELEASE_ENABLED=true`. Publish the next version through CI within npm's two-day validation window. If desired, manually dispatch `release.yml` **on the bootstrap tag** with `publish=true` to verify existing package checks and finish the GitHub Release; this does not validate an unused npm publisher because no new npm version is uploaded.

## Prepare subsequent versions

From a working tree containing the intended changes:

```sh
pnpm release:version 0.1.0-alpha.1 --dry-run
pnpm release:version 0.1.0-alpha.1
pnpm release:check
pnpm typecheck
pnpm test:release
```

The script updates:

- `[workspace.package].version` in `Cargo.toml`;
- all seven exact internal workspace dependency requirements;
- all ten local workspace entries in `Cargo.lock`, including the private adapter and tooling;
- the public npm version and all five optional native dependency versions.

It validates before and after the change and rolls back modified version files if post-update validation fails. It rejects downgrades, malformed versions and build metadata. It creates no commits or tags and performs no publication. The private root npm package and documentation package retain their independent tooling versions. The pnpm lockfile currently contains no release package versions, so it stays unchanged; the script fails if release versions become locked and require a new update strategy. Do not use `bumpp` alone for coordinated releases.

Review the version diff, update release notes as needed, and merge the release commit. Then create and push its tag:

```sh
git tag -a v0.1.0-alpha.1 -m 'Release 0.1.0-alpha.1'
git push origin v0.1.0-alpha.1
```

Use a stable version such as `0.1.0` to publish to `latest`. Publishing an alpha does not move `latest`.

## What the workflow verifies

`release.yml` calls `ci.yml` and `npm.yml` through `workflow_call` and waits for both. This includes Rust tests, Clippy, rustfmt, schema/compatibility/syntax checks, Rust 1.88 checks, engine comparisons and all ten npm consumer receipts. A separate release-script job runs on normal npm CI as well.

The freeze job rechecks tarball hashes, clean-source receipts and commit identity, copies exactly six npm tarballs and compiles all eight packaged Rust crates using Cargo's temporary registry. It uploads one immutable `release-artifacts-ATTEMPT` bundle with 30-day retention. A freeze retry uses a new attempt name, so a successful upload followed by a failed job cleanup cannot collide with the previous artifact. Publisher jobs download the bundle by the exact **artifact ID returned by the successful freeze job**, not by a name search or latest-attempt guess. They do not rebuild npm artifacts.

Rust publication checks its source manifest, archive VCS metadata and registry SHA-256 checksums, and compares each archive repackaged by Cargo against the frozen archive before upload. npm publication compares SHA-512 integrity and validates the frozen complete matrix. Registry reads use bounded backoff for network failures, 429 and 5xx responses; 401/403 fail immediately, and only 404 means absent. Existing versions with different bytes require investigation and a new version, not an overwrite. A CLI upload error is reconciled by checking the expected immutable bytes in the registry before deciding whether the job failed; a registry write is never automatically repeated.

The Rust CLI is installed back from crates.io with an exact `=VERSION` requirement after publication. npm installs an exact root version from the registry with an empty cache and tests API output and CLI version before advancing channels. The npm execution preflight validates Node/npm versions and, inside GitHub Actions, the presence of the OIDC request environment provided by `id-token: write`. It does not use `npm whoami`, which does not validate OIDC permissions. A successful upload of a new version remains the actual proof of the platform's trust configuration. A GitHub Release and artifact attachments are created only after both publisher jobs succeed. The workflow never uses `workflow_run` or `pull_request_target` to publish.

## Dry runs and recovery

Run **Actions → Release → Run workflow** with `publish` unchecked to validate any selected ref and freeze artifacts. To resume publication by dispatch, select the version **tag** as the ref, check `publish`, and ensure the repository variable is enabled. Branch refs cannot publish.

For a partial publication, prefer **Re-run failed jobs** on the original run. This retains the successful freeze job's artifact ID, even when the run attempt increases. Matching existing versions are skipped and only missing versions are uploaded; completed channel promotions are skipped, and missing promotions resume after the complete set and consumer checks pass. All six channels are checked before uploads and again before promotion. Neither a newer platform channel nor a newer root channel can be moved backwards. The root channel is promoted last, after each platform promotion is confirmed.

Publish one version tag at a time. GitHub's default concurrency behavior preserves the running release and one pending release; additional pending runs can replace that pending run. If a tag's pending run is canceled, dispatch that tag explicitly after the active release finishes.

Do not rebuild or overwrite frozen artifacts after any package was uploaded. A full rerun may produce different native bytes; conflicting integrity stops publication. If frozen artifacts have expired or bytes no longer match, prepare a new version. Never force-move an existing version tag to different source.

The two registries and multiple npm tags do not form a transaction. A registry outage can leave crates.io ahead of npm, or some platform tags advanced while the root channel remains old. Preserve the run and artifacts, fix the external failure, and resume. Do not promote the root channel manually for a partial release.

## References

- [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Cargo workspace packaging](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [Official crates.io authentication action](https://github.com/rust-lang/crates-io-auth-action)
- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
