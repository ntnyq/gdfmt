# 0.1.0 release checklist

[简体中文](RELEASE_CHECKLIST.zh-CN.md)

Track the remaining work for the first stable release here. The current feature
baseline includes the CLI, Rust and Node APIs, WASM, stdio LSP, desktop and Web
VS Code extensions, and the bilingual documentation playground.

Check an item only after reviewing its evidence. Record the candidate commit,
workflow/report links and artifact hashes in the release PR or release notes.
Historical acceptance records do not validate a new revision. This checklist
does not change CI; publication and recovery follow [the publishing guide](RELEASING.md).

## Scope and public contracts

The [style/Schema review](docs/research/0.1.0-style-schema-review.md) and both P2
follow-ups are complete. The reviewed defaults and Schema contract are frozen;
candidate validation below still needs evidence for the final release revision.

The [CLI/API/version review](docs/research/0.1.0-api-contract-review.md) is also
complete, with argument routing, strict JavaScript validation, WASM metadata and
version synchronization regressions in place.

The [desktop/Web editor review](docs/research/0.1.0-editor-contract-review.md) is
complete. Configuration limits, mixed-EOL rejection and unsupported ranges now
have additional regressions; trust and line-ending guidance are aligned.

The [full project-trial review](docs/research/0.1.0-project-trials-review.md)
passed on the pre-candidate alpha.6 working tree: 606 scripts, 2,420 accepted layouts,
zero unexplained differences and all 606 approved fingerprints unchanged.
A comparator colon-classification gap was fixed with two regressions. This is
dirty-working-tree evidence; the candidate validation below records the clean rc.1 rerun.

- [x] Review and freeze the default formatting style and configuration schema;
      inspect representative diffs for comments, lambdas, wrapping, literal
      spelling and protected regions before accepting any output changes.
- [x] Review the CLI arguments/exit codes, Rust and JavaScript APIs, diagnostics
      and version synchronization for the 0.1.0 contract. Keep the documented
      Godot 4.3–4.7 profiles and compatibility boundaries explicit.
- [x] Confirm desktop and Web editor behavior and document their differences,
      including Web configuration restrictions, mixed line endings and the lack
      of range formatting. Keep project analysis and semantic fixes outside scope.
- [x] Align integer-valued decimal/exponent configuration input with Schema
      acceptance, including overrides and browser configuration helpers, and
      extend the regression matrix (review finding P2).
- [x] Complete the bilingual line-width limitations and generated Schema
      description for expressions/chains and preserved content (review finding P2).
- [x] Run the full pinned project trial on the current baseline, review every
      category combination, fix the overbroad inner-class colon classification
      and rerun without recording new fingerprints. Archive source/report hashes
      and keep the existing default formatting style.

## Candidate validation

**`0.1.0-rc.1` is published** on crates.io and npm `next`/`candidate`, with independent
extension version **`0.2.0`** and unchanged default style. [PR #1](https://github.com/ntnyq/gdfmt/pull/1)
was squash-merged into `1329b72dc26179921739e4c2342ffa0bf7b7a314`, tagged as
`v0.1.0-rc.1`. The [publication report](docs/research/0.1.0-rc.1-publication.md)
records exact tag-bound CI, artifact ID/hashes, fifteen registry payloads and real
installed Rust/npm consumers. All npm `latest` tags retain their previous values.

The [candidate CI report](docs/research/0.1.0-rc.1-ci.md) and
[initial preparation](docs/research/0.1.0-rc.1-preparation.md) remain historical
snapshots of their own source revisions. Candidate publication does not validate
future stable 0.1.0 artifacts. No stable release or Marketplace upload occurred.

- [x] Prepare rc.1 source versions and bilingual candidate notes; review current
      changes, fix the version-dependent legacy recovery test fixture, and run
      local release checks. Keep clean-revision/full-matrix acceptance separate.

- [x] Pass the complete [Rust CI](.github/workflows/ci.yml): rustfmt, Clippy,
      schema, compatibility and syntax coverage, tests with and without default
      features on Linux/macOS/Windows, and Rust 1.88 workspace/WASM checks.
- [x] Pass parser and formatter conformance against all five pinned Godot
      profiles. Review separately classified analyzer boundaries; see
      [engine comparisons](CONTRIBUTING.md#engine-comparisons).
- [x] Pass `pnpm typecheck`, `pnpm release:check` and `pnpm test:release`, plus
      the complete [npm matrix](.github/workflows/npm.yml). Require all ten
      consumer receipts and `pnpm verify:npm`; a `--local` pass is insufficient.
- [x] Pass [WASM and editor acceptance](.github/workflows/hosts.yml): installed
      WASM tarball in Node/Chromium/module Worker, desktop Extension Hosts,
      Godot Tools coexistence, Alpine smoke check, and Web lifecycle and real
      Web Extension Host tests without cross-origin isolation. Include isolated
      installed-VSIX smoke checks on desktop runners; staged Web host tests do
      not establish an end-user browser installation route.
- [x] Pass `pnpm typecheck:docs` and `pnpm test:docs`, including both languages
      of the playground. Repeat the installation/example/API/editor documentation
      audit against the chosen candidate revision and its versioned artifacts.
- [x] Rerun the full [pinned project trials](tests/project-trials/README.md) on
      the candidate, beyond the ordinary `pnpm test:trials` guards. Review changed
      diffs and fingerprints, resolve unexplained changes, then rerun without
      `--record-review`. Attach the summary and tested revision. This is a manual
      0.1.0 acceptance item; the full trial remains optional in ordinary CI.

The exact release tag passed the full CI, eight packaged crate builds, six VSIX
targets and complete project trial, retaining all 606 review entries and zero
unexplained differences. Original frozen artifact `12544141250` contains seven npm
tarballs, eight crates and six VSIX files. All fifteen registry downloads and all
24 GitHub assets match its hashes. Linux CI and independent macOS ARM64 registry
consumers passed; native/WASM parity covers 1,465 cases. Final stable-tag freezing
and publication gates remain separate.

## Documentation and release notes

The [bilingual documentation review](docs/research/0.1.0-documentation-review.md)
was completed for the prior `0.1.0-alpha.6` baseline. The rc.1 preparation updates
source-candidate labels and rechecks examples, declarations and editor versions.
After rc.1 publication, installation commands use the published rc.1 and point to
its registry acceptance; stable 0.1.0 remains unreleased.

The 0.1.0 release notes are prepared in [English](docs/releases/0.1.0.md) and
[Simplified Chinese](docs/zh/releases/0.1.0.md), including host/profile support,
default style, a `gdformat` migration sequence and known limitations. They remain
drafts until final acceptance and publication; include them in the GitHub Release
body during the publication-completion review.

The [Marketplace/VSIX review](docs/research/0.1.0-editor-distribution-review.md)
recommends frozen GitHub VSIX delivery for core 0.1.0, with Marketplace stable
publication tracked independently. Gallery already has numeric extension 0.1.0
as a six-target prerelease containing alpha.6 in the inspected macOS/Web manifests.
rc.1 selected numeric 0.2.0; six frozen prerelease VSIX files are now available
on its GitHub Release. No Gallery upload occurred. Recheck before
uploading; a Gallery prerelease upload of 0.2.0 would require another numeric
version for stable. Web installation and packaged trust differences are explicit.

- [x] Correct the bilingual claims that extensions/LSP are absent, remove the
      first-release qualifier from WASM source-build instructions, and clarify
      parser/engine responsibility and expression whitespace preservation.
- [x] Audit bilingual installation instructions, configuration examples, API
      declarations, editor setup and Web limitations against the 0.1.0 contract
      baseline; record the reviewed version and verification evidence.
- [x] Prepare user-facing 0.1.0 release notes covering supported hosts/profiles,
      formatting style, migration from `gdformat`, and known limitations. Do not
      claim full Godot compatibility or runtime equivalence.
- [x] Record the 0.1.0 distribution recommendation: frozen VSIX delivery as the
      baseline, a separate Marketplace stable plan with a new numeric extension
      version, and native/Web installation routes. Existing workflows attach
      VSIX files; Marketplace publication is not a core-release gate.

## Final version and publication readiness

**Committed source: core 0.1.0, extension 0.2.1; unpublished.** [PR #3](https://github.com/ntnyq/gdfmt/pull/3)
and the [clean-source CI report](docs/research/0.1.0-ci.md) record complete branch/PR
CI, exact clean-revision project trials and artifact identities. PR merge and the
final tag-bound release freeze remain pending. Published rc.1 receipts keep their
own source identity. Defaults/API/Schema are unchanged; registry installation still
uses rc.1. No stable tag or Marketplace upload was created.

The [registry/environment review](docs/research/0.1.0-registry-readiness-review.md)
confirms existing ownership and historical alpha.6 Trusted Publishing for all
fifteen packages. On 2026-10-23, `RELEASE_ENABLED=true` and the `release`
environment permits `v*` tags. The subsequent protection work configured `main`
and active tag ruleset [25720774](https://github.com/ntnyq/gdfmt/rules/25720774),
with administrator/maintainer exceptions and verified API readback. The applied
policy is recorded in [.github/repository-protection.json](.github/repository-protection.json).
The owner subsequently confirmed setup. rc.1 now exercises all fifteen Trusted
Publishers and npm direct-publish/dist-tag rights in a successful new release;
public registry identity, hashes and real consumers are recorded in the
[publication report](docs/research/0.1.0-rc.1-publication.md). The
[initial per-package review](docs/research/0.1.0-trusted-publisher-review.md) retains
historical private-read/session limitations. Email/2FA/access and bootstrap-token
revocation have not been independently read back, so the remaining owner-account
evidence stays unchecked. Optional Trusted Publishing Only hardening is not a new gate.

- [x] Complete the read-only ownership, historical Trusted Publishing and
      `release` environment audit for all eight crates and seven npm packages,
      including `@gdfmt/wasm`; record evidence, individual current-configuration
      query results, account-session limits and remaining owner actions.
- [x] Implement and verify `main` protection and the `v*` tag ruleset: PRs,
      29 required checks, administrator emergency bypass and Admin/Maintain
      tag exceptions, including recovery tags. Record the policy and API/role
      readback without publishing or creating refs.
- [ ] Archive the remaining authenticated owner-account evidence: email verification,
      2FA/access and bootstrap-token revocation. rc.1 already proves all fifteen
      functional Trusted Publishers and npm direct-publish/dist-tag permissions;
      do not repeat configuration unless those settings change.
- [x] Confirm the chosen candidate's publishing setup with a successful new
      release and installed registry consumers. rc.1 run `40051909889` and the
      publication report record all fifteen payloads, exact installed consumers,
      `next` promotion and unchanged `latest` tags.
- [x] Review and retain the pinned npm 11.21.0 runtime and scoped dist-tag
      workaround. This read-only review does not meet the publishing guide's
      removal criteria.
- [x] Run `pnpm release:version 0.1.0 --dry-run`, apply the update and review
      Cargo/npm versions, the lockfile, five optional native dependencies and
      extension core version. `pnpm release:check`, `pnpm typecheck` and all
      32 release regressions passed. Select independent numeric extension **0.2.1**,
      above rc.1's 0.2.0 and unused in the 2026-10-24 Gallery snapshot. The
      [initial preparation report](docs/research/0.1.0-preparation.md) preserves
      historical uncommitted receipts; clean-source CI acceptance is recorded below.
- [x] Commit and review the stable preparation in [PR #3](https://github.com/ntnyq/gdfmt/pull/3),
      pass the complete Rust/npm/host branch and PR CI matrices, and rerun pinned
      project trials on the exact clean revision. The [CI report](docs/research/0.1.0-ci.md)
      records all ten native consumer receipts, six VSIX targets, eight packaged
      crates and the validation rehearsal. Final-head checks are recorded in the PR.
- [ ] Merge PR #3 in a separately authorized step, verify a clean final checkout
      and default-branch ancestry, and recheck the intended release environment/
      `RELEASE_ENABLED` setting immediately before pushing `v0.1.0`.

## Publication completion

These remaining items apply to the final stable 0.1.0 release. rc.1's tag-bound
freeze, registry consumers and GitHub prerelease are complete in the report above.
Preserve each release's original frozen bundle when recovering a partial release.

- [ ] Confirm the final tag's full CI matrices and freeze job passed. Record the
      successful run/artifact ID and matching revision, receipts and hashes for
      eight crate archives, seven npm tarballs and six VSIX files.
- [ ] Verify exact-version crates.io installation and npm native/WASM API and CLI
      consumers, matching registry integrity, and advancement of all seven npm
      packages to `latest` before marking the release complete.
- [ ] Verify the GitHub Release contains the release notes and frozen artifacts;
      update bilingual installation instructions to the stable channel and
      confirm the documentation/playground deployment serves the intended version.

## Follow-up work outside the 0.1.0 gate

- Repeatable performance/memory baselines and budgets, continuous fuzzing, and
  broader real-project coverage.
- Additional native platforms or Bun, Deno and general WASI hosts.
- Additional editor capabilities such as range formatting, or broader Web
  configuration support.
- Marketplace stable rollout after publisher/authentication checks, a fresh
  numeric version, six-target publication/recovery and installed Gallery acceptance;
  the existing prerelease listing remains distinct from the final core release.

These are follow-up candidates, not commitments for 0.1.0. Lint rules, semantic
auto-fixes, project analysis and runtime validation remain outside the formatter's
current scope; see [compatibility](docs/reference/compatibility.md).
