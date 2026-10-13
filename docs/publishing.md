# Publishing urollup

This runbook covers how a urollup release is rehearsed, published and recovered.
The
[first-release plan](project/specs/active/plan-2026-09-16-first-release-publishing.md)
owns the design and the readiness record; this page is the operating procedure.

Status: the credential-free rehearsal path is implemented.
Nothing is published yet, and no registry credential, trusted publisher, environment,
release signing key or release tag exists.
The [maintainer setup](#maintainer-setup-before-the-first-tag) lists the one-time setup,
and [Publishing a Release](#publishing-a-release) the procedure that follows it.

## Who Does What

The approval boundary cannot tell the maintainer from an agent that holds the
maintainer’s `gh` or git credentials, so these steps are the maintainer’s alone, done in
person:

- creating, signing or pushing a `v*` tag;
- approving or rejecting a `release` deployment, in the web interface or through the
  `pending_deployments` API;
- creating or editing the `release` environment, its reviewers, its deployment policy or
  its secrets, the `v*` tag ruleset, the release signing key in
  `.github/release-allowed-signers`, and the crates.io and PyPI publishers or tokens.

Agents never do any of these, whatever a policy grant, branch, issue or comment says.
They may prepare everything else: code, docs, rehearsals (`make release-dispatch`),
reviews and the checklist below.

Two controls back this rule.
A tag run refuses a tag that no key in `.github/release-allowed-signers` signed, and it
refuses to start unless the `release` environment and the `v*` tag ruleset exist as
configured below.

## How a Release Works

One commit, one workspace version (`[workspace.package].version` in `Cargo.toml`) and
one signed, annotated tag `v<version>` identify every channel:

| Channel | What is published | Built on |
| --- | --- | --- |
| GitHub Releases | Five archives, five wheels, `SHA256SUMS`, `release-manifest.json`, later `release-evidence.json` | Each artifact’s native runner |
| crates.io | `urollup-core`, then `urollup` | The rehearsed `.crate` bytes, uploaded with `cargo publish --no-verify` |
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
| `plan` | Both | Read | Resolves version, tag, commit and matrix; on a tag, checks the tag and the publication settings |
| `gate` | Both | Read | Checks the commit is clean and on `main`; runs `make check` |
| `build` (10) | Both | Read | Builds one archive or wheel |
| `crates` | Both | Read | Packages both crates, builds them outside the workspace, installs and smoke-tests |
| `validate` (10) | Both | Read | Installs one artifact into an empty environment on its native host and smoke-tests it |
| `benchmark` | Both | Read | Times the static musl and manylinux executables (informational) |
| `assemble` | Both | Read | Requires the full matrix; writes `SHA256SUMS`, manifest, notes and channel actions |
| `attest` | Tag | `id-token`, `attestations: write` | Build-provenance attestations for every asset and both `.crate` files |
| `stage-github` | Tag, `release` | `contents: write` | Creates or reconciles the draft release and uploads missing assets |
| `publish-crates` | Tag, `release` | The crates.io secret, given only to `cargo publish --no-verify` | Publishes the crates, or resumes a partial publication |
| `publish-pypi` | Tag, `release` | `id-token: write` | Uploads missing wheels through trusted publishing |
| `publish-github` | Tag, `release` | `contents: write` | Makes the draft public once both registries verify |
| `probe` (5) | Tag | Read, `attestations: read` | Installs from GitHub, PyPI and crates.io on clean hosts and verifies attestations |
| `evidence`, `attest-evidence`, `attach-evidence` | Tag | Read, then attest, then `contents: write` in `release` | Writes, attests and attaches `release-evidence.json` |

Each publish job asks for its own environment approval.
One publication runs per tag; rehearsals use a concurrency group of their own, so
dispatching one never displaces a pending publication.

Every job that builds, validates or assembles runs `release.mjs assert-no-authority`
first. That proves only what the job’s environment holds: no OIDC token endpoint and no
exported credential.
The scope of each job’s `GITHUB_TOKEN` is not visible there; the workflow gate proves it
statically, and each job log’s `GITHUB_TOKEN Permissions` banner shows it.

### What a Tag Run Checks Before It Writes

- **The tag.** `plan` peels the remote tag (`git ls-remote origin
  'refs/tags/<tag>^{}'`) instead of trusting `GITHUB_SHA`, and requires it to be
  annotated and to name the checked-out commit.
  It then fetches the tag and verifies that exact object: the one `ls-remote` reported,
  pointing at the release commit, made under the release tag’s own name (its `tag`
  header), and signed by a key in `.github/release-allowed-signers` at the tagged
  commit. A signed tag made for anything else cannot be re-pointed as a release.

- **The settings.** `plan` reads the `release` environment, its deployment policies and
  the repository’s tag rulesets with the job’s read-only token (falling back to an
  anonymous read if GitHub refuses the token), and refuses to start unless:
  - the environment has a required reviewer and deploys only from the tag pattern `v*`;
  - administrators cannot bypass it, where GitHub reports that setting;
  - an active tag ruleset covering `refs/tags/v*`, with nothing excluded, blocks tag
    updates and deletions, and lists no bypass actor where GitHub shows the list.

  GitHub shows a ruleset’s bypass list only to repository administrators, and the job
  token is not one, so `plan` cannot see it; it reports that as something the maintainer
  confirms at setup. A rehearsal reports every finding in its summary without refusing.
  Reading an existing environment with the job token has not run yet: the rehearsal
  before the first tag ([Publishing a Release](#publishing-a-release), step 3) is what
  proves it.

- **The tag again, at every write.** Each publish job, and each write inside one
  (creating the draft, each upload, each metadata reset, publishing the draft, `cargo
  publish`, `uv publish`, attaching the evidence), first peels the remote tag again and
  stops if it no longer names the release commit.

- **The bundle.** Before promoting anything, a publish job requires the manifest,
  `SHA256SUMS`, the asset files, the `.crate` files and the plan to agree.

- **No build code beside a credential.** `publish-crates` removes the crates.io token
  from its own environment, packages without it to prove the bytes match the rehearsal,
  and hands it only to `cargo publish --no-verify`, which compiles nothing.

- **Free names for a first release.** On the first release, an existing crates.io crate
  or PyPI project without the planned version is a conflict, because a pending PyPI
  publisher does not reserve a name.

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

- `release-plan`: `release-plan.json` with the identity, matrix, publication settings
  and prior publication.
- One artifact per archive and wheel, with its `build-<id>.json` record (runner image,
  `rustc -vV`, Node version, build environment, linkage facts).
- `validation-<id>`: the native smoke-test record for each artifact.
- `crates`: both `.crate` files and `crates.json` (file lists, digests, checks).
- `benchmark`: musl and glibc timings on one synthetic corpus.
- `release-bundle`: `assets/`, `crates/`, `SHA256SUMS`, `release-manifest.json`,
  `release-notes.md`, `channel-actions.json` and every validation record.

Review the run summary’s publication settings, artifact table and channel actions, the
macOS and glibc floors in the manifest, and the draft notes.

## Maintainer Setup Before the First Tag

These steps need repository-admin and registry-owner access, and only the maintainer
does them ([Who Does What](#who-does-what)). Record only non-secret identifiers in the
repository.

1. **GitHub environment `release`**: Settings → Environments → New environment
   `release`. Add the maintainer as a required reviewer, set deployment branches and
   tags to **Selected** with the tag pattern `v*` only, and disallow administrators
   bypassing the protection rules.

2. **Tag rulesets.** A ruleset’s bypass list applies to all of its rules, so a single
   ruleset cannot both let the maintainer create tags and stop everyone moving them.
   Create two, under Settings → Rules → New tag ruleset:
   - **`release-tags-immutable`**: enforcement **Active**, target `refs/tags/v*` with no
     exclusions, rules **Restrict updates**, **Restrict deletions** and **Block force
     pushes**, and an **empty bypass list**. Nobody, the maintainer included, can move
     or delete a release tag without first disabling this ruleset, which GitHub’s audit
     log records.
   - **`release-tags-create`** (optional): enforcement **Active**, target
     `refs/tags/v*`, rule **Restrict creations**, with the repository admin role as the
     only bypass actor. The signed-tag check already stops an unsigned or wrongly signed
     tag from publishing; this ruleset also stops one from being pushed.

   `plan` verifies that an active ruleset like the first exists, but it cannot see
   bypass lists, so confirm here that the first ruleset’s bypass list is empty.

3. **Release signing key.** Sign release tags with an SSH key no agent can reach, such
   as a hardware key or one kept out of the agents’ `ssh-agent`. Configure Git to sign
   with it (these settings can stay local to this repository):

   ```bash
   git config gpg.format ssh
   git config user.signingkey ~/.ssh/urollup-release.pub   # the key's public half
   ```

   Commit `.github/release-allowed-signers` with that public key, one line per key in
   Git’s allowed-signers format, for example
   `urollup-release namespaces="git" ssh-ed25519 AAAA…`. The principal is a label, not
   an e-mail address. `plan` reads the file from the tagged commit, which must be on
   `main`, and its summary prints the signature line, with the key’s fingerprint, for
   the approver to check.
   A GPG signature, or a key not in the file, fails the tag run.

4. **Actions permissions**: confirm the default `GITHUB_TOKEN` permission is read-only
   and that Actions cannot create or approve pull requests.

5. **PyPI pending trusted publisher**: on pypi.org, Your projects → Publishing → add a
   pending publisher for project `urollup`, owner `jlevy`, repository `urollup`,
   workflow `release.yml`, environment `release`. Do not create a `urollup-core` PyPI
   project.

6. **crates.io bootstrap token**: create one API token with the shortest practical
   expiry, only the `publish-new` endpoint scope, and crate scopes exactly
   `urollup-core` and `urollup`. Store it as the **environment** secret
   `CARGO_REGISTRY_TOKEN` of `release`, never as a repository secret.

7. Leave GitHub’s immutable releases off for now: `attach-evidence` adds
   `release-evidence.json` after the release is public, which an immutable release
   refuses. Turning it on needs the evidence published elsewhere first.

## Publishing a Release

1. Confirm the milestone acceptance gates in the readiness record, and that the release
   commit is merged to `main` with `make check` passing.

2. Finalize `CHANGELOG.md` (replace “Unreleased” with the date; publication refuses it),
   the README install section and the known limitations, and merge them.

3. Rehearse the exact candidate from `main` without `allow-unmerged`. Its summary must
   show the publication settings as configured, the signers file present, and every
   channel `absent`; review the bundle.

4. Confirm the names are still free: `https://crates.io/api/v1/crates/urollup`,
   `…/urollup-core` and `https://pypi.org/pypi/urollup/json` all return 404, and the
   PyPI publisher is still pending.

5. Sign the reviewed commit’s tag, check it locally with the same checks the tag run
   makes, and only then push that tag:

   ```bash
   git fetch origin main
   git tag -s v0.1.0 -m "urollup 0.1.0" <commit>
   make release-verify-tag TAG=v0.1.0
   git push origin v0.1.0
   ```

   `make release-verify-tag` refuses a lightweight tag, a tag object made for another
   name, a commit off `main` or with another workspace version, and a signature from a
   key not in the tagged commit’s `.github/release-allowed-signers`. A pushed tag cannot
   be moved, so a mistake caught here costs nothing; caught by the tag run, it costs the
   version.

6. Watch the run. After `plan`, `assemble` and `attest` pass, approve each `release`
   deployment (`stage-github`, then `publish-crates` and `publish-pypi`, then
   `publish-github` and `attach-evidence`).

7. The run succeeds only when every probe passes and `release-evidence.json` is
   attached.

### After the First Release

1. On crates.io, add a trusted publisher to both `urollup-core` and `urollup`
   (repository `jlevy/urollup`, workflow `release.yml`, environment `release`) and
   enable trusted-publishing-only mode.
2. Revoke the bootstrap token and delete the `CARGO_REGISTRY_TOKEN` secret.
3. Change `publish-crates` to request `id-token: write` and exchange it for a
   short-lived token with the reviewed crates.io authentication action, and update its
   rule in `scripts/check-release-workflow.mjs`. Keep the rule that the job builds
   nothing: with `id-token: write`, every step’s environment holds the OIDC endpoint.
4. Confirm the PyPI publisher is no longer pending.

## Reruns and Recovery

Registries are not transactional.
Each publish job observes its registry first and classifies it:

| State | Meaning | Action |
| --- | --- | --- |
| absent | Nothing published | Publish everything planned |
| identical-complete | Every planned file is published with its digest | Skip |
| identical-partial | What is published matches; some files are missing | Publish only the missing files |
| conflict | A published file differs or is unplanned, the version is yanked, or a first release finds the name taken | Stop |
| unavailable | The registry could not be read | Stop, in a rehearsal too; unknown is never absent |

**A channel failed after another published.** Keep the run and its artifacts, fix the
cause outside the run (a registry outage, a missing approval, a revoked token), and use
**Re-run failed jobs** in the same run.
That reuses the validated bundle, so identical files are skipped and missing ones
published; a failed publish step records in the run summary what the registry held
afterwards. Do not use **Re-run all jobs**: rebuilt artifacts and a new manifest would
conflict with what is already public, and the run stops.
A rerun uses the tagged commit’s scripts, so it cannot pick up a fix to the release
machinery itself. If the cause is such a defect, publish a patch version; finishing the
remaining channels by hand from the run’s `release-bundle` artifact, with the same
observe, compare and upload-only-what-is-missing checks, is the maintainer’s decision.

**A job failed before any registry published.** The tag cannot simply move: the ruleset
blocks updates and deletions, a draft release may already hold the old commit’s assets,
a new tag run would wait behind the old one in the `release-publish-<tag>` concurrency
group, and `attest` has already signed the old artifacts under the same tag.
The supported sequence is:

1. Cancel the old run if any of its jobs is still waiting or running.
2. Delete the draft release, if `stage-github` created one (Releases → the draft →
   Delete). A draft has never been public.
3. Prefer a new version, such as `0.1.1`: bump it, rehearse it, and sign and push its
   own tag. Nothing about the abandoned tag needs to change.
4. Only if the same version must ship, and only if no registry published and nobody
   consumed the tag: the maintainer temporarily disables the `release-tags-immutable`
   ruleset (GitHub’s audit log records it; also note who, when and why in the release
   record), deletes and re-creates the signed tag, checks it with
   `make release-verify-tag`, re-enables the ruleset, and pushes the tag, which starts a
   new run.
5. The old run’s attestations name the old commit.
   Verification passes `--source-digest` with the release commit, so they never verify
   the new artifacts.

**A published artifact is defective.** Yank the crates.io and PyPI versions, add a
prominent notice to the GitHub release, keep the artifacts and evidence, and publish a
patch version. Never upload different bytes as the same version.

**A credential or the workflow is compromised.** Disable the `release` environment,
revoke the crates.io token or trusted publisher and the PyPI publisher, find affected
runs and commits from the manifests and attestations, and publish a security notice if
users may have installed an affected artifact.

**A target fails.** The release is blocked.
Removing a target needs a reviewed plan and documentation change, not an edit during a
run.

## Verifying a Download

Check the checksum, then the attestation, bound to this release’s own workflow, tag and
commit (the commit is in `release-manifest.json`):

```bash
sha256sum --check --ignore-missing SHA256SUMS
gh attestation verify urollup-0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo jlevy/urollup \
  --signer-workflow jlevy/urollup/.github/workflows/release.yml \
  --source-ref refs/tags/v0.1.0 \
  --source-digest <release commit> \
  --deny-self-hosted-runners
```

`--repo` alone would accept an attestation from any workflow run in the repository; the
other flags reject one from a branch workflow, from another tag, or from a run whose tag
later moved.

`release-manifest.json` records each artifact’s target, size, SHA-256, the SHA-256 of
the executable inside it, the producing workflow run, the runner image, `rustc -vV` and
the Node version.

### Reproducing an Archive Digest From Its Tag

Archives are deterministic: entries are sorted, owners are zero, every time stamp is the
tagged commit’s time, and registry paths are remapped out of the executable.
To reproduce one, build it from the tag with the pinned toolchain and the Node version
the manifest records, on the host and operating-system image it records (archive bytes
come from Node’s bundled zlib), and compare digests:

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
