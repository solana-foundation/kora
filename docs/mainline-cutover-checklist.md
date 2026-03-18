# Mainline Cutover Checklist

This checklist moves Kora from long-lived `release/*` development to a mainline model where:

- `main` is the only long-lived development branch.
- Audited status is tied to commit/tag snapshots, not branch names.

## Preconditions

1. Freeze feature merges into `release/2.2.0`.
2. Announce cutover window to maintainers and reviewers.

## 1) Merge Branch Histories Into `main`

```bash
git fetch origin
git checkout -b cutover/mainline-unification origin/main
git merge --no-ff origin/release/2.2.0
# Resolve conflicts
make check
make test
```

Open PR:

- Source: `cutover/mainline-unification`
- Target: `main`
- Include summary of resolved conflicts and test outputs.

## 2) Apply Policy/CI Changes (this PR)

Merge the PR that:

- Removes `release/*` branch filters in CI workflows.
- Updates branch policy docs (`README`, `CONTRIBUTING`).
- Adds `audits/AUDIT_STATUS.md`.

## 3) Update GitHub Ruleset Scope

Current repository ruleset includes both `refs/heads/main` and `refs/heads/release/*`.
After cutover, scope it to `refs/heads/main` only.

Verification command:

```bash
gh api repos/solana-foundation/kora/rulesets/12299344 --jq '.conditions.ref_name.include'
```

Expected result after update:

```json
["refs/heads/main"]
```

## 4) Retire Long-Lived Release Branch

After `main` contains the merged release history and CI is green:

1. Mark `release/2.2.0` as deprecated in repository docs/channels.
2. Lock or delete `release/2.2.0` remote branch.
3. Remove any stale automations that still target `release/*`.

## 5) Publish and Audit Discipline Going Forward

1. Continue merging feature/fix PRs to `main`.
2. Cut prereleases with semver prerelease tags from `main` (for example `v2.3.0-beta.1`).
3. Cut stable audited tags from audited commits only (for example `v2.3.0`).
4. Update `audits/AUDIT_STATUS.md` whenever a new audit/mitigation review lands.

## 6) Post-Cutover Validation

```bash
# Confirm CI triggers no longer reference release/*
rg -n 'release/\\*' .github/workflows -S

# Confirm docs point feature work to main
rg -n 'target.*release/|audited code only|release/X.Y.Z' README.md CONTRIBUTING.md -S
```
