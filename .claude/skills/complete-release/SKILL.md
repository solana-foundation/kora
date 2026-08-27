---
name: complete-release
description: "Reviewer workflow for completing a Kora mainline release after the release PR is merged. Squash-merges the PR, detects whether Rust and/or TypeScript changed, then triggers the correct publish workflows on main. For hotfix releases, publish from hotfix/* before merging back. Use when the user says 'complete the release' or 'merge and publish' (mainline only)."
---

# Kora Complete Release

Run after a mainline release PR is approved: squash-merge it, then trigger the publish workflows
on `main`.

## Merge

Find the release PR (`gh pr list --base main --search "chore: release"`) and confirm the number
with the user before merging — this publishes to crates.io and npm, which cannot be undone.
Squash-merge, delete the branch, then pull `main`.

## Decide which workflows to run

Publish only what the release actually changed. Inspect the merge commit: paths under
`crates/`, `Cargo.toml`, `Cargo.lock` or `CHANGELOG.md` mean Rust; paths under `sdks/` mean
TypeScript. A combined release triggers both.

```bash
gh workflow run "Publish Rust Crates" --ref main \
  -f publish-kora-lib=true -f publish-kora-cli=true -f create-github-release=true

gh workflow run "Publish TypeScript SDK" --ref main \
  -f publish-to-npm=true -f create-github-release=true
```

Watch with `gh run list --workflow "<name>" --limit 3`. Verify against `git tag` and
`npm view @solana/kora version` once green.

## Constraints

- **Mainline only.** Hotfixes publish from `hotfix/*` *before* merging back to `main`. The publish
  workflows accept `main` and `hotfix/*` refs only.
- `kora-lib` publishes before `kora-cli` (cli depends on lib). CI already waits ~30s between them
  for crates.io indexing; don't run them as separate manual dispatches.
- On a partial failure, re-dispatch with the already-succeeded booleans set to `false` rather than
  re-running the whole workflow — a repeat publish of an existing version fails the run.
- TS prereleases publish under the `beta` npm tag, stable under `latest`.
