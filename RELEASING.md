# Publishing gdfmt

[简体中文](RELEASING.zh-CN.md)

The Rust workspace and public npm packages share one version. A `vVERSION` tag runs the full Rust CI and the complete npm matrix, freezes the artifacts, publishes crates.io, publishes npm, and then creates a GitHub Release. Ordinary branch pushes and pull requests only validate changes.

Registry writes are disabled until the GitHub repository variable `RELEASE_ENABLED` is exactly `true`. Manual workflow runs default to validation only. No registry credentials are needed to build, test or freeze artifacts.

Track the first stable release's remaining acceptance work in the
[0.1.0 release checklist](RELEASE_CHECKLIST.md).

## Current 0.1.0 readiness

**Core 0.1.0 is published** on crates.io and npm `latest`; six frozen GitHub
VSIX files use independent extension **0.2.1**, without the prerelease flag.
[PR #3](https://github.com/ntnyq/gdfmt/pull/3) was squash-merged into
`d98db6b0463d91e6449f2c84dd62026ceb0698cd` and tagged `v0.1.0`.
[Release run 40118540445](https://github.com/ntnyq/gdfmt/actions/runs/40118540445)
validated and froze that revision before publishing both registries.
The [publication report](docs/research/0.1.0-publication.md) records the original
bundle, fifteen registry payloads, real consumers and all 24 GitHub assets.
The clean-source CI rehearsal and rc.1 reports retain their historical identities.
All seven npm `next` tags retain rc.1; `candidate` and `latest` point to 0.1.0.
Marketplace stable rollout and the remaining owner-account evidence stay separate.

The [registry and environment review](docs/research/0.1.0-registry-readiness-review.md)
records the initial read-only snapshot and the branch/tag protection follow-up
from 2026-10-23. All eight crates and seven npm
packages already exist under public owner/maintainer `ntnyq`, and alpha.6 was
published through Trusted Publishing in
[release run 39749770445, attempt 2](https://github.com/ntnyq/gdfmt/actions/runs/39749770445/attempts/2).
The bootstrap instructions below apply to initial setup, not to the next release.

The `release` environment exists and permits `v*` tags; the repository variable
`RELEASE_ENABLED` is already `true`. `main` branch protection and the active
[Version tags ruleset](https://github.com/ntnyq/gdfmt/rules/25720774) are now
configured and verified. Their expected settings are recorded in
[repository-protection.json](.github/repository-protection.json), with maintainer
exceptions described below. Environment deployment admission remains separate
from tag creation, modification and deletion protection.

The [initial Trusted Publisher/account review](docs/research/0.1.0-trusted-publisher-review.md)
records historical private-configuration read limits. The rc.1 release now proves
all fifteen functional Trusted Publishers and npm direct-publish/dist-tag rights:
every crate's public publishing identity matches the release run/commit, and every
npm package records GitHub Trusted Publishing and provenance. Owner confirmation
of setup remains recorded; email/2FA and bootstrap-token revocation have not been
independently read back. Keep that remaining account-evidence item separate from
the completed candidate publication/consumer gate. Optional Trusted Publishing
Only hardening is not a new release requirement.

For a read-only local review, use `pnpm release:check` and `pnpm test:release`.
GitHub environment, variable, branch and ruleset API reads and `npm trust list`
are also read-only; registry administration reads require suitable authentication.
Do not treat a new `v*` tag push as validation only while the gate is enabled.
A manual workflow run with `publish: false` remains validation only. The historical
protection follow-up changed only branch/tag settings. The later rc.1 tag and
publication are recorded separately in the publication report.

## Release units

Rust packages publish in this dependency order:

```text
gdfmt_text → gdfmt_syntax → gdfmt_parser → gdfmt_formatter
→ gdfmt → gdfmt_config → gdfmt_workspace → gdfmt_cli
```

`gdfmt_napi`, `gdfmt_wasm` and `xtask` remain unpublished. The CLI crate is `gdfmt_cli`; its installed executable is `gdfmt`:

```sh
cargo install gdfmt_cli --version '=VERSION' --locked
```

The npm release contains `@gdfmt/wasm`, `@gdfmt/gdfmt` and these optional native packages:

```text
@gdfmt/darwin-arm64
@gdfmt/darwin-x64
@gdfmt/linux-x64-gnu
@gdfmt/linux-x64-musl
@gdfmt/win32-x64-msvc
```

All five platform versions must match the root package exactly. Prereleases use npm's `next` channel; stable versions use `latest`. Each package is first uploaded under `candidate`. Accepted npm uploads can take several minutes to appear publicly. Platform candidates are submitted before waiting up to ten minutes for visibility; the root is uploaded only after every platform is confirmed. The release channels are advanced only after all seven versions are visible with matching integrity and an isolated registry consumer passes the Node API, WASM and CLI smoke checks. Platform channels advance before the root channel. The `candidate` tag is retained for inspection.

## One-time platform setup

### GitHub

In `ntnyq/gdfmt`:

1. Enable GitHub Actions and allow the actions used by the workflows. Public repositories should use GitHub-hosted runners, including the macOS Intel and arm64 runners used by the native matrix.
2. Create an environment named **`release`**. Allow version tags matching `v*`. Leave required reviewers unset for automatic publication, or configure them if publication should require approval.
3. Protect the default branch and use a tag ruleset to restrict creation, modification and deletion of `v*` tags to release maintainers. Release preflight also requires the tagged commit to be reachable from the default branch.
4. Leave **Settings → Secrets and variables → Actions → Variables → `RELEASE_ENABLED`** unset until bootstrap and Trusted Publisher setup are complete. Set it to **`true`** when ready.

No `NPM_TOKEN`, `CARGO_REGISTRY_TOKEN` or personal GitHub token secret is needed for subsequent releases. The crates.io job exchanges OIDC for a temporary token; npm uses OIDC directly. Only the final GitHub Release job receives `contents: write`.

#### Applied branch and tag policy

The reviewed REST request bodies live in
[`.github/repository-protection.json`](.github/repository-protection.json).
They document the applied settings; workflows do not automatically apply this file.

- **`main`:** require a PR, an up-to-date branch, resolved conversations and all
  29 existing CI/npm/host checks, bound to the GitHub Actions app (ID `15368`).
  Required approvals are **0**, with no code-owner or last-push approval
  requirement, so the sole maintainer can merge a passing PR. Force pushes and
  deletion are disabled. `enforce_admins: false` retains the administrator's
  emergency bypass; ordinary changes should still use passing PRs.
- **`refs/tags/v*`:** active ruleset **25720774** restricts creation, update and
  deletion. Only repository **Admin (role ID 5)** and **Maintain (role ID 2)**
  have `always` bypass. Write access alone is insufficient. This includes
  `v-recovery-*`; there is no recovery-pattern exclusion or Actions/app bypass.

These exceptions let maintainers create release and recovery tags without
turning protection off. They do not make retagging a published version a recovery
procedure: keep published tags immutable, preserve the original frozen bundle,
and use a new recovery ref when the documented procedure needs one. Release
creation uses `--verify-tag` against an existing tag, so `GITHUB_TOKEN` needs no
tag-creation bypass. `release` environment admission, OIDC permissions and
`RELEASE_ENABLED` are unchanged.

The 29 required check names were matched to successful checks on `main` at
`6c11445a329b688f27fd6693a3c6314ffeedce64` before configuration. When renaming
jobs or changing matrices, update both this JSON and the live required checks
to avoid leaving PRs waiting for obsolete names. Branch-protection and ruleset
API readback, plus GraphQL role-name readback, verified the applied policy;
no test tag, force push, workflow dispatch or publication was used.

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
2. Create or obtain publishing access to the **`@gdfmt` scope**, including the main package **`@gdfmt/gdfmt`**. GitHub repository ownership does not grant npm scope ownership.
3. Ensure the publishing account can create and update all seven public packages. Complete bootstrap below using interactive authentication.
4. In **each of the seven packages**, add a GitHub Actions Trusted Publisher:

| Field                | Value         |
| -------------------- | ------------- |
| Organization or user | `ntnyq`       |
| Repository           | `gdfmt`       |
| Workflow filename    | `release.yml` |
| Environment          | `release`     |
| Allow `npm publish`  | Enabled       |
| Allow `npm dist-tag` | Enabled       |

Enter only the workflow filename, including `.yml`. New configurations can default to staged publishing; this workflow needs direct publication and separate dist-tag permission. A newly configured npm publisher must successfully publish a **new version within two days**. Skipping an already-published version during a retry does not validate that new trust configuration. If it expires, recreate it before the next release.

The publishing job pins Node 24.0.0 and the npm version recorded in `scripts/release/npm-compat.json` (currently 11.21.0), which supports OIDC publication and dist-tag management. Both execution preflight and the temporary dist-tag patch reject other npm versions before writes. These are publishing requirements; the consumer matrix still verifies Node 22.13.0 and 24.0.0. Once OIDC works, npm package settings can require 2FA and disallow token publication.

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

Replace the example tag with the actual version. Replace `FREEZE_ATTEMPT` with the successful freeze attempt (for example, `1`); a later publisher-only retry does not change that artifact name. The bundle contains `npm/`, `crates/` and `editors/`. Downloaded `npm/release.json` records the commit, clean-source status, complete matrix, selected files and SHA-512 integrity. `crates/crates.json` binds all eight Rust archives to the same commit and SHA-256 checksums, and the verifier also checks each archive's Cargo VCS metadata. The selected native tarballs come from the Node 24.0.0 runs; the Node 22.13.0 runs independently test the same root tarball. Native artifacts built by the two Node jobs are not required to be byte-identical.

5. When ready to perform the first actual publication, use the pinned npm 11.21.0, authenticate and explicitly execute the plans. Node 24.0.0's originally bundled npm is older, so upgrade it first:

```sh
npm install --global npm@11.21.0 --ignore-scripts
cargo login
npm login
pnpm release:publish:crates --execute
pnpm release:publish:npm --execute
```

The two `--execute` commands perform real registry writes. They require a clean checkout, including staged and untracked files. The npm publisher invokes npm's JavaScript CLI with the active Node runtime instead of executing `npm.cmd`; for a custom npm installation prefix, set `GDFMT_NPM_CLI` to that installation's `npm/bin/npm-cli.js`. The scripts publish missing packages in order, verify registry checksums, and stop on conflicts. crates.io packaging must resolve against crates.io; registry source replacement can interfere with Cargo's temporary workspace registry. Use a clean release environment if local Cargo configuration changes registry resolution.

crates.io allows a burst of five new crates, then restores one new-crate publication every ten minutes. Bootstrapping these eight crates can therefore require about thirty minutes waiting for quota recovery. On HTTP 429, wait until the server's stated retry time and rerun the publisher with the original frozen artifacts; matching published crates are skipped. Updates to existing crates have a separate version-publication quota; see [crates.io publishing rate limits](https://crates.io/docs/rate-limits).

Local npm publication and Trusted Publisher configuration may each require browser-based two-factor authentication. A successful `npm login` does not remove these checks. Use an interactive terminal for manual publication and follow npm's authentication prompts.

6. Configure the Trusted Publishers above and enable `RELEASE_ENABLED=true`. Publish the next version through CI within npm's two-day validation window. Finish the bootstrap GitHub Release using the original frozen bundle after both registries pass installation checks. Do not dispatch a full bootstrap rerun after upload: rebuilding native archives can change their bytes. Skipping an existing version cannot validate an unused npm publisher.

## Prepare subsequent versions

From a working tree containing the intended changes:

```sh
pnpm release:version 0.1.0-rc.2 --dry-run
pnpm release:version 0.1.0-rc.2
pnpm release:check
pnpm typecheck
pnpm test:release
```

The script updates:

- `[workspace.package].version` in `Cargo.toml`;
- all seven exact internal workspace dependency requirements;
- all eleven local workspace entries in `Cargo.lock`, including private adapters and tooling;
- both public npm versions, all five optional native dependency versions and the VS Code `gdfmtVersion` field.

It validates before and after the change and rolls back modified version files if post-update validation fails. It rejects downgrades, malformed versions and build metadata. It creates no commits or tags and performs no publication. The private root npm package and documentation package retain their independent tooling versions. The pnpm lockfile currently contains no release package versions, so it stays unchanged; the script fails if release versions become locked and require a new update strategy. Do not use `bumpp` alone for coordinated releases.

Version replacement targets the actual `[workspace.package]` field, leaving
commented version examples intact. The VS Code Marketplace version is an
independent numeric SemVer with no leading zeros or prerelease suffix; the script
updates only `gdfmtVersion`, so bump the numeric extension version separately when
needed. Configuration and JSON-result `schemaVersion: 1` are format contracts,
independent of Cargo/npm release versions.

Review the version diff, update release notes as needed, and merge the release commit.

Prepared 0.1.0 notes are available in [English](docs/releases/0.1.0.md) and
[Simplified Chinese](docs/zh/releases/0.1.0.md). Recheck them against the final
candidate and remove the draft status only when publication is complete. The
current workflow uses GitHub-generated notes; incorporate this bilingual text
into the final GitHub Release body as part of the publication-completion review.
These source documents are not automatically used as the release body.

Then create and push the release tag:

```sh
git tag -a v0.1.0-rc.2 -m 'Release 0.1.0-rc.2'
git push origin v0.1.0-rc.2
```

Use a stable version such as `0.1.0` to publish to `latest`. Publishing a prerelease, including rc.1, does not move `latest`.

## What the workflow verifies

`release.yml` calls `ci.yml`, `npm.yml` and `hosts.yml` through `workflow_call` and waits for all three. This includes Rust tests, Clippy, rustfmt, schema/compatibility/syntax checks, Rust 1.88 checks, engine comparisons and all ten npm consumer receipts. A separate release-script job runs on normal npm CI as well.

The freeze job rechecks tarball hashes, clean-source receipts and commit identity, copies exactly seven npm tarballs and compiles all eight packaged Rust crates using Cargo's temporary registry. It uploads one immutable `release-artifacts-ATTEMPT` bundle with 30-day retention. A freeze retry uses a new attempt name, so a successful upload followed by a failed job cleanup cannot collide with the previous artifact. Publisher jobs download the bundle by the exact **artifact ID returned by the successful freeze job**, not by a name search or latest-attempt guess. They do not rebuild npm artifacts.

Rust publication checks its source manifest, archive VCS metadata and registry SHA-256 checksums, and compares each archive repackaged by Cargo against the frozen archive before upload. npm publication compares SHA-512 integrity and validates the frozen complete matrix. Registry reads use bounded backoff for network failures, 429 and 5xx responses; 401/403 fail immediately, and only 404 means absent. Existing versions with different bytes require investigation and a new version, not an overwrite. A CLI upload error is reconciled by checking the expected immutable bytes in the registry before deciding whether the job failed; a registry write is never automatically repeated.

The Rust CLI is installed back from crates.io with an exact `=VERSION` requirement after publication. npm installs an exact root version from the registry with an empty cache and prefers fresh metadata. Missing install metadata (`ETARGET`/404) is retried for up to ten minutes because the install index can lag the immutable version endpoint; authentication and other install failures stop immediately. It tests native/WASM API output and CLI version before advancing channels. The npm execution preflight validates Node/npm versions and, inside GitHub Actions, the presence of the OIDC request environment provided by `id-token: write`. It does not use `npm whoami`, which does not validate OIDC permissions. A successful upload of a new version remains the actual proof of the platform's trust configuration. A GitHub Release and artifact attachments are created only after both publisher jobs succeed. The workflow never uses `workflow_run` or `pull_request_target` to publish.

## WASM and VS Code artifacts

For core 0.1.0, use the six frozen GitHub Release VSIX assets as the delivery
baseline; Marketplace stable publication is an independent follow-up, not a
core-release gate. The [distribution review](docs/research/0.1.0-editor-distribution-review.md)
records both routes and remaining work. Public Gallery metadata on 2026-10-23
already lists numeric extension `0.1.0` as a six-target prerelease; inspected
macOS/Web manifests contain core `0.1.0-alpha.6`. It is not a stable core release.

The rc.1 GitHub Release provides six frozen VSIX files with numeric extension
`0.2.0` and core `0.1.0-rc.1`, marked as prerelease. This publication did not upload
to the Gallery or change the Marketplace listing. Recheck availability before
any later upload and rebuild/test/freeze all six variants. If `0.2.0` is uploaded
as a Gallery prerelease, select another unused numeric version for stable.
The stable GitHub Release now provides numeric extension **0.2.1** with core
**0.1.0** across all six frozen VSIX targets, without a prerelease flag. This
release did not upload to Marketplace. Recheck Gallery availability before any
later upload; see the [publication report](docs/research/0.1.0-publication.md).
Do not reuse the existing Gallery prerelease version or publish the raw source
directory. Verify publisher access/authentication, publish frozen VSIX bytes through
a separately authorized process and verify installed stable consumers for every target.

Desktop `pnpm test:vscode` also installs its VSIX in an isolated temporary
profile, checks the installed extension/core versions and bundled server bytes,
and formats stdin with that installed server. Freezing desktop artifacts requires
`installedVsixPassed: true` alongside Extension Host acceptance; Alpine keeps
its native smoke check.

The browser workflow installs the actual WASM tarball in Node and Chromium,
compares all shared fixtures across five profiles, and tests a module Worker
without isolation headers. Its `npm-wasm` receipt must match the release commit,
clean-source status and exact SHA-512 before freezing. The WASM package is
uploaded with platform candidates before the native facade and receives the same
`next` or `latest` channel. Configure its npm Trusted Publisher separately.

`hosts.yml` also creates platform VSIX files. Freeze requires native version
checks on all five targets and Extension Host evidence on four desktop runners.
Alpine uses the native binary smoke check; Electron cannot run on musl. Linux's
desktop host additionally tests with Godot Tools enabled. Each VSIX records its
core version, numeric extension version, revision and SHA-512 in `editors.json`.
GitHub Releases attach those frozen bytes. This workflow does not publish to the
VS Code Marketplace. Increment the extension's numeric Marketplace version when
preparing a Marketplace update; it cannot carry Cargo's prerelease suffix.

`hosts.yml` also packages a separate browser-only Web VSIX from the same WASM
package. Freeze requires its Chromium Web Extension Host receipt with
`webExtensionHostPassed: true` and `crossOriginIsolated: false`, the same clean
revision and exact SHA-512. All six VSIX files are attached to the GitHub Release;
none is automatically published to the Marketplace. Local Web validation is
`pnpm test:vscode:web`; `pnpm package:vscode:web` alone does not record acceptance.

For native installation, select the asset matching the local/remote extension
host and use **Extensions: Install from VSIX**; check **gdfmt: Show Version** after
manual updates or rollback. Browser-only VS Code 1.96.4 does not offer this local
VSIX action without a local/remote extension-management server. Its existing
Gallery prerelease or developer sideloading are separate routes; staged Web host
acceptance does not prove end-user Gallery/VSIX installation. See
[installation details](docs/guide/integrations.md#vs-code).

`release:version` updates the WASM npm version and the extension's `gdfmtVersion`;
the numeric extension version is maintained independently. `gdfmt_wasm` is excluded
from crates.io packaging. Historical six-package frozen releases remain valid
when recovery selects their original source checkout; the new WASM requirement
is applied only to source revisions that contain the WASM host.

## Dry runs and recovery

Release commands log their start, completion, and a heartbeat every 15 seconds. npm commands have a three-minute deadline, individual HTTP requests use a 30-second timeout, and npm's automatic HTTP retries are disabled so registry writes are not silently repeated. Cargo packaging and publication have ten-minute command deadlines. Registry reads log each retry; npm visibility and install-metadata waits have ten-minute budgets, and channel confirmation has a one-minute budget. A timed-out write is reconciled against the registry before the publisher stops or continues.

The temporary npm launcher canonicalizes scoped package slash escapes to `%2F` for dist-tag reads and writes, retaining npm's original OIDC/OTP handling. During alpha.5 recovery, lowercase `%2f` requests stalled while equivalent uppercase requests succeeded; the responsible network or registry layer was not established. The patch depends on npm's private `DistTag.fetchTags` method, so it only supports the exact npm version in `scripts/release/npm-compat.json`; other commands do not load that private module. Do not widen the version constraint on an upgrade.

Remove the patch when a replacement npm runtime, without the launcher patch, passes scoped dist-tag reads and a controlled tag write using GitHub OIDC in the existing `release` environment, and a fresh registry install passes API/CLI checks. Use a disposable test channel for the write and verify its registry result. In the same change, pin that validated runtime in `npm-compat.json`, delete the private-method override, update the launcher tests and record the validation evidence. Until those checks pass, retain the current pin and patch.

npm version checks first read the package metadata used by installs and verify the recorded integrity. If that index has not listed the version yet, the publisher checks the immutable version endpoint before treating it as absent. Existing versions verified during preflight are not polled again; newly uploaded versions still wait for matching integrity before publication continues.

Run **Actions → Release → Run workflow** with `publish` unchecked to validate any selected ref and freeze artifacts. Before any uploads, dispatch can start publication by selecting the version **tag** as the ref, checking `publish`, and enabling the repository variable. Branch refs cannot publish. After an upload, resume the original run instead of dispatching a rebuild.

For a partial publication, prefer **Re-run failed jobs** on the original run. This retains the successful freeze job's artifact ID, even when the run attempt increases. Matching existing versions are skipped and only missing versions are uploaded; completed channel promotions are skipped, and missing promotions resume after the complete set and consumer checks pass. All six channels are checked before uploads and again before promotion. Neither a newer platform channel nor a newer root channel can be moved backwards. The root channel is promoted last, after each platform promotion is confirmed.

If recovery needs a newer publisher script, run it from a clean, committed tooling checkout and pass a separate clean checkout of the original version tag. Download the original successful freeze bundle without rebuilding it, then inspect the plan and execute it:

```sh
pnpm release:publish:npm --source ../gdfmt-release-source --artifacts target/release/npm
pnpm release:publish:npm --source ../gdfmt-release-source --artifacts target/release/npm --execute
```

`--source` selects the source to validate, not a replacement revision for the artifacts. Its exact Git commit, version, package metadata, tarball hashes and original complete matrix must still match. Both checkouts must be clean for execution. Use the npm runtime and interactive authentication requirements described above. Create the GitHub Release from the original bundle only after both registries and installation checks have succeeded.

To retain GitHub Trusted Publishing when newer tooling is required, tag the committed tooling revision with a unique `v-recovery-*` tag and dispatch `release.yml` on that ref with `publish=true` and the original `recovery_artifact` ID. The original run, version tag and commit are derived from that artifact, so they cannot be supplied inconsistently. Recovery tags do not trigger a normal release. The recovery job requires both commits to be on the default branch history, checks that the exact artifact belongs to the original version's Release run with successful freeze and Rust publication jobs, and downloads that artifact by ID. It then runs the npm publisher against a separate original-source checkout, using the existing `release` environment and OIDC trust. It performs no rebuild and creates no GitHub Release; finish the latter from the original bundle after verifying both registries. Never move the original version tag or relax environment policies for recovery.

Publish one version tag at a time. GitHub's default concurrency behavior preserves the running release and one pending release; additional pending runs can replace that pending run. If a tag's pending run is canceled, dispatch that tag explicitly after the active release finishes.

Do not rebuild or overwrite frozen artifacts after any package was uploaded. A full rerun may produce different native bytes; conflicting integrity stops publication. If frozen artifacts have expired or bytes no longer match, prepare a new version. Never force-move an existing version tag to different source.

The two registries and multiple npm tags do not form a transaction. A registry outage can leave crates.io ahead of npm, or some platform tags advanced while the root channel remains old. Preserve the run and artifacts, fix the external failure, and resume. Do not promote the root channel manually for a partial release.

## References

- [Cargo publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Cargo workspace packaging](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [Official crates.io authentication action](https://github.com/rust-lang/crates-io-auth-action)
- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
