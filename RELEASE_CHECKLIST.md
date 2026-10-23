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

The **`0.1.0-rc.1`** source is committed in [PR #1](https://github.com/ntnyq/gdfmt/pull/1),
with independent extension version **`0.2.0`** and unchanged default style.
The [candidate CI report](docs/research/0.1.0-rc.1-ci.md) binds acceptance to source
`5d95932e63a1d5199916466013bf557561279fe5`; its linked artifact evidence records
clean-source receipts, workflow/artifact IDs and hashes. The final documentation
follow-up must also pass complete CI on its own PR head, recorded in the PR.

The [preparation report](docs/research/0.1.0-rc.1-preparation.md) remains a historical
`cleanSource: false` WIP snapshot. No tag or publication occurred. Candidate
acceptance below does not validate final 0.1.0 artifacts at a future tagged revision.

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

Clean rc.1 evidence is recorded in the candidate CI report: ten native consumers,
installed WASM, all six VSIX targets (four desktop installed-VSIX checks), both
editor hosts, bilingual Playground, eight clean packaged crate builds and all five
engine profiles. The full project trial retains all 606 review entries and zero
unexplained differences. Complete `pnpm verify:npm` and a local `pnpm release:prepare`
rehearsal verified seven npm tarballs, eight crates and six VSIX files. These are
source-candidate validation artifacts; tag-bound freezing and publication gates
remain separate.

## Documentation and release notes

The [bilingual documentation review](docs/research/0.1.0-documentation-review.md)
was completed for the prior `0.1.0-alpha.6` baseline. The rc.1 preparation updates
source-candidate labels and rechecks examples, declarations and editor versions;
public registry commands retain published versions until a real upload.

The 0.1.0 release notes are prepared in [English](docs/releases/0.1.0.md) and
[Simplified Chinese](docs/zh/releases/0.1.0.md), including host/profile support,
default style, a `gdformat` migration sequence and known limitations. They remain
drafts until final acceptance and publication; include them in the GitHub Release
body during the publication-completion review.

The [Marketplace/VSIX review](docs/research/0.1.0-editor-distribution-review.md)
recommends frozen GitHub VSIX delivery for core 0.1.0, with Marketplace stable
publication tracked independently. Gallery already has numeric extension 0.1.0
as a six-target prerelease containing alpha.6 in the inspected macOS/Web manifests.
The rc.1 preparation selects numeric 0.2.0 after a fresh availability check;
local VSIX files are prereleases and no Gallery upload occurred. Recheck before
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

The [registry/environment review](docs/research/0.1.0-registry-readiness-review.md)
confirms existing ownership and historical alpha.6 Trusted Publishing for all
fifteen packages. On 2026-10-23, `RELEASE_ENABLED=true` and the `release`
environment permits `v*` tags. The subsequent protection work configured `main`
and active tag ruleset [25720774](https://github.com/ntnyq/gdfmt/rules/25720774),
with administrator/maintainer exceptions and verified API readback. The applied
policy is recorded in [.github/repository-protection.json](.github/repository-protection.json).
The owner subsequently reported that publishing configuration is complete.
Record this as owner confirmation, separate from the earlier unauthenticated
query results. Field-by-field authenticated evidence and account-readiness
confirmations have not been captured in this repository; the checklist item
below retains that evidence requirement, and candidate publication remains
a separate gate.

The [per-package trust/account review](docs/research/0.1.0-trusted-publisher-review.md)
records the earlier eight 403 and seven E401 results, npm account/access failures
and signed-out sessions. These historical query limits do not contradict the
owner’s later setup confirmation. Archive authenticated, non-secret evidence of
all fifteen bindings, both npm permissions and email/2FA/access/token revocation. Public `trustpub_only: false` on all eight crates is optional-hardening
state, not a new acceptance gate. Keep authenticated review unchecked until
the actual settings have been read.

- [x] Complete the read-only ownership, historical Trusted Publishing and
      `release` environment audit for all eight crates and seven npm packages,
      including `@gdfmt/wasm`; record evidence, individual current-configuration
      query results, account-session limits and remaining owner actions.
- [x] Implement and verify `main` protection and the `v*` tag ruleset: PRs,
      29 required checks, administrator emergency bypass and Admin/Maintain
      tag exceptions, including recovery tags. Record the policy and API/role
      readback without publishing or creating refs.
- [ ] Obtain an authenticated owner review of current Trusted Publisher bindings
      for all fifteen packages, npm direct-publish/dist-tag permissions, account
      readiness and bootstrap-token revocation. Resolve mismatches or expired
      new configurations before candidate publication.
- [ ] Confirm the chosen candidate's publishing setup with a successful new
      release and installed registry consumers; historical alpha.6 evidence
      cannot close this gate.
- [x] Review and retain the pinned npm 11.21.0 runtime and scoped dist-tag
      workaround. This read-only review does not meet the publishing guide's
      removal criteria.
- [ ] Run `pnpm release:version 0.1.0 --dry-run`, apply the version update, review
      the synchronized manifests/lockfile/core extension version, and run
      `pnpm release:check`, `pnpm typecheck` and `pnpm test:release`. Select the
      independent unused numeric extension version before freezing all six VSIX
      files (currently 0.2.0 for rc.1, subject to future Gallery availability).
- [ ] Merge the final release commit, verify a clean checkout and default-branch
      ancestry, and confirm the intended `RELEASE_ENABLED` setting before pushing
      `v0.1.0` according to the publishing guide.

## Publication completion

These items are checked during publication. Preserve the original frozen bundle
when recovering a partial release.

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
