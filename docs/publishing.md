# Publishing urollup

This runbook covers how a urollup release is rehearsed, published and recovered.
The
[first-release plan](project/specs/active/plan-2026-09-16-first-release-publishing.md)
owns the design and the readiness record; this page is the operating procedure.

Status: the credential-free rehearsal path is implemented.
Nothing is published yet, and no registry credential, trusted publisher, environment or
release tag exists. The [maintainer checklist](#maintainer-setup-before-the-first-tag)
lists the one-time setup, and [Publishing a Release](#publishing-a-release) the
procedure that follows it.

## How a Release Works

One commit, one workspace version (`[workspace.package].version` in `Cargo.toml`) and
one tag `v<version>` identify every channel:

| Channel | What is published | Built on |
| --- | --- | --- |
| GitHub Releases | Five archives, five wheels, `SHA256SUMS`, `release-manifest.json`, later `release-evidence.json` | Each artifact’s native runner |
| crates.io | `urollup-core`, then `urollup` | `cargo publish --locked --workspace` from the tag |
| PyPI | The five `urollup` binary wheels, no sdist | The validated wheels, unchanged |

[`.github/workflows/release.yml`](../.github/workflows/release.yml) has two entry paths:

- **Rehearsal** (`workflow_dispatch`): runs the plan, `make check`, every build, native
  validation, the Cargo packaging rehearsal and assembly, then stops.
  It has no write permission, no OIDC token and no secret, so it cannot publish.
- **Publication** (a pushed `v*` tag): runs the same jobs, then attests the validated
  artifacts and enters the protected `release` environment for each publish job.

Every decision lives in a tested script (`scripts/release*.mjs`); the workflow only
calls them. `make release-check`, which `make check` and CI run, tests that logic and
proves the workflow’s authority boundaries (`scripts/check-release-workflow.mjs`).

### Jobs and Their Authority

| Job | Runs in | Authority | What it does |
| --- | --- | --- | --- |
| `plan` | Both | Read | Resolves version, tag, commit and matrix; records prior publication |
| `gate` | Both | Read | Checks the commit is clean and on `main`; runs `make check` |
| `build` (10) | Both | Read | Builds one archive or wheel; proves it holds no credential |
| `crates` | Both | Read | Packages both crates, builds them outside the workspace, installs and smoke-tests |
| `validate` (10) | Both | Read | Installs one artifact into an empty environment on its native host and smoke-tests it |
| `benchmark` | Both | Read | Times the static musl and manylinux executables (informational) |
| `assemble` | Both | Read | Requires the full matrix; writes `SHA256SUMS`, manifest, notes and channel actions |
| `attest` | Tag | `id-token`, `attestations: write` | Build-provenance attestations for every asset |
| `stage-github` | Tag, `release` | `contents: write` | Creates the draft release and uploads missing assets |
| `publish-crates` | Tag, `release` | `CARGO_REGISTRY_TOKEN` secret | Publishes the crates, or resumes a partial publication |
| `publish-pypi` | Tag, `release` | `id-token: write` | Uploads missing wheels through trusted publishing |
| `publish-github` | Tag, `release` | `contents: write` | Makes the draft public once both registries verify |
| `probe` (5) | Tag | Read | Installs from GitHub, PyPI and crates.io on clean hosts |
| `evidence`, `attest-evidence`, `attach-evidence` | Tag | Read, then attest, then `contents: write` in `release` | Writes, attests and attaches `release-evidence.json` |

Each publish job asks for its own environment approval.

## Rehearsing

### Locally

```bash
make release-check      # release logic tests, workflow authority, version identity
make release-rehearsal  # this host's archive and wheel plus both Cargo packages
```

`make release-rehearsal` builds, inspects, installs and smoke-tests this host’s
artifacts under `target/release-rehearsal`. It needs uv, and on Linux the wheel build
needs Docker. A dirty tree is allowed and recorded, because a local rehearsal is never a
release. Only CI has all five hosts and the assemble step.

### In CI

```bash
make release-dispatch                      # the pushed HEAD of this branch
make release-dispatch ALLOW_UNMERGED=true  # a commit that is not on main yet
```

This calls `gh workflow run release.yml` with the commit, the workspace version and
`allow-unmerged`. From the Actions tab, use **Run workflow** on the branch whose head is
the candidate. The plan refuses a commit that is not the head of the dispatched ref, and
a version that is not the workspace version.
`allow-unmerged` lets the gate pass for a commit that is not on `main`; such a run
records that the commit is not releasable.

GitHub dispatches only workflows it has registered, which happens once `release.yml` is
on the default branch.
Before that, a branch push that triggers the workflow registers it; the plan then
refuses that push because it is not a tag.

### Reviewing a Rehearsal

The run keeps every output for 30 days:

- `release-plan`: `release-plan.json` with the identity, matrix and prior publication.
- One artifact per archive and wheel, with its `build-<id>.json` record (runner image,
  `rustc -vV`, build environment, linkage facts).
- `validation-<id>`: the native smoke-test record for each artifact.
- `crates`: both `.crate` files and `crates.json` (file lists, digests, checks).
- `benchmark`: musl and glibc timings on one synthetic corpus.
- `release-bundle`: `assets/`, `SHA256SUMS`, `release-manifest.json`,
  `release-notes.md`, `channel-actions.json` and every validation record.

Review the run summary’s artifact table, the channel actions, the macOS and glibc floors
in the manifest, and the draft notes.

## Maintainer Setup Before the First Tag

These steps need repository-admin and registry-owner access.
Record only non-secret identifiers in the repository.

1. **GitHub environment `release`**: Settings → Environments → New environment
   `release`. Add the maintainer as a required reviewer, set deployment branches and
   tags to **Selected** with the tag pattern `v*` only, and disallow administrators
   bypassing the protection rules.
2. **Tag ruleset**: Settings → Rules → new tag ruleset targeting `v*` that restricts
   creation to the maintainer and blocks updates and deletions, so a published tag
   cannot move.
3. **Actions permissions**: confirm the default `GITHUB_TOKEN` permission is read-only
   and that Actions cannot create or approve pull requests.
4. **PyPI pending trusted publisher**: on pypi.org, Your projects → Publishing → add a
   pending publisher for project `urollup`, owner `jlevy`, repository `urollup`,
   workflow `release.yml`, environment `release`. Do not create a `urollup-core` PyPI
   project.
5. **crates.io bootstrap token**: create one API token with the shortest practical
   expiry, only the `publish-new` endpoint scope, and crate scopes exactly
   `urollup-core` and `urollup`. Store it as the **environment** secret
   `CARGO_REGISTRY_TOKEN` of `release`, never as a repository secret.
6. Optionally enable immutable releases in the repository settings, so a published
   release’s assets cannot change.

## Publishing a Release

1. Confirm the milestone acceptance gates in the readiness record, and that the release
   commit is merged to `main` with `make check` passing.

2. Finalize `CHANGELOG.md` (replace “Unreleased” with the date; publication refuses it),
   the README install section and the known limitations, and merge them.

3. Rehearse the exact candidate from `main` without `allow-unmerged`, and review the
   bundle.

4. Tag the reviewed commit and push only that tag:

   ```bash
   git tag -a v0.1.0 -m "urollup 0.1.0" <commit>
   git push origin v0.1.0
   ```

5. Watch the run. After `assemble` and `attest` pass, approve each `release` deployment
   (`stage-github`, then `publish-crates` and `publish-pypi`, then `publish-github` and
   `attach-evidence`).

6. The run succeeds only when every probe passes and `release-evidence.json` is
   attached.

### After the First Release

1. On crates.io, add a trusted publisher to both `urollup-core` and `urollup`
   (repository `jlevy/urollup`, workflow `release.yml`, environment `release`) and
   enable trusted-publishing-only mode.
2. Revoke the bootstrap token and delete the `CARGO_REGISTRY_TOKEN` secret.
3. Change `publish-crates` to request `id-token: write` and exchange it for a
   short-lived token with the reviewed crates.io authentication action, and update its
   rule in `scripts/check-release-workflow.mjs`.
4. Confirm the PyPI publisher is no longer pending.

## Reruns and Recovery

Registries are not transactional.
Each publish job observes its registry first and classifies it:

| State | Meaning | Action |
| --- | --- | --- |
| absent | Nothing published | Publish everything planned |
| identical-complete | Every planned file is published with its digest | Skip |
| identical-partial | What is published matches; some files are missing | Publish only the missing files |
| conflict | A published file differs or is unplanned, or the version is yanked | Stop |
| unavailable | The registry could not be read | Stop; unknown is never absent |

- **A job failed before anything published**: fix the problem on a new commit, rehearse
  it, and move the tag only if nothing was published or consumed from it.
- **A channel failed after another published**: keep the run and its artifacts, fix the
  cause, and use **Re-run failed jobs** in the same run.
  That reuses the validated bundle, so identical files are skipped and missing ones
  published. Do not use **Re-run all jobs**: rebuilt artifacts and a new manifest would
  conflict with what is already public, and the run stops.
- **A published artifact is defective**: yank the crates.io and PyPI versions, add a
  prominent notice to the GitHub release, keep the artifacts and evidence, and publish a
  patch version. Never upload different bytes as the same version.
- **A credential or the workflow is compromised**: disable the `release` environment,
  revoke the crates.io token or trusted publisher and the PyPI publisher, find affected
  runs and commits from the manifests and attestations, and publish a security notice if
  users may have installed an affected artifact.
- **A target fails**: the release is blocked.
  Removing a target needs a reviewed plan and documentation change, not an edit during a
  run.

## Verifying a Download

```bash
sha256sum --check --ignore-missing SHA256SUMS
gh attestation verify urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz --repo jlevy/urollup
```

`release-manifest.json` records each artifact’s target, size, SHA-256, the SHA-256 of
the executable inside it, the producing workflow run, the runner image and `rustc -vV`.

### Reproducing an Archive Digest From Its Tag

Archives are deterministic: entries are sorted, owners are zero, every time stamp is the
tagged commit’s time, and registry paths are remapped out of the executable.
To reproduce one, build it from the tag with the pinned toolchain on the host and
operating-system image the manifest records, and compare digests:

```bash
git checkout v0.1.0
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct) \
  node scripts/release.mjs build --artifact archive-linux-x86_64 --out /tmp/reproduce
sha256sum /tmp/reproduce/urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz
```

A different linker or SDK can change the executable; dispatching a rehearsal of the
tagged commit runs the same build on the same runner labels.
Compare `executable_sha256` first, then the archive digest.

<!-- This document follows common-doc-guidelines.md.
See github.com/jlevy/practical-prose and review guidelines before editing.
-->
