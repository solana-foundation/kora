---
name: complete-release
description: "Reviewer workflow for completing a Kora mainline release after the release PR is merged. Squash-merges the PR, detects whether Rust and/or TypeScript changed, then triggers the correct publish workflows on main. For hotfix releases, publish from hotfix/* before merging back. Use when the user says 'complete the release' or 'merge and publish' (mainline only)."
---

# Kora Complete Release

Run this after a mainline release PR has been approved. It squash-merges the PR and triggers the appropriate CI publish workflows on `main`.

---

## Step 1 — Identify the release PR

```bash
# List open release PRs
gh pr list --base main --search "chore: release" --json number,title,headRefName
```

Confirm the correct PR number with the user before proceeding.

---

## Step 2 — Squash-merge the PR

```bash
gh pr merge "${PR_NUMBER}" --squash --delete-branch
```

---

## Step 3 — Pull latest main

```bash
git checkout main && git pull origin main
```

---

## Step 4 — Detect what changed

```bash
# Find the merge commit
merge_commit=$(git log --oneline -1 --format="%H")

# Check for Rust changes
rust_changed=$(git diff-tree --no-commit-id -r "$merge_commit" \
  --name-only | grep -E '^(Cargo\.toml|Cargo\.lock|CHANGELOG\.md|crates/)' | head -1)

# Check for TypeScript changes
ts_changed=$(git diff-tree --no-commit-id -r "$merge_commit" \
  --name-only | grep -E '^sdks/' | head -1)

echo "Rust changed: ${rust_changed:-no}"
echo "TS changed:   ${ts_changed:-no}"
```

---

## Step 5 — Trigger Rust publish workflow (if Rust changed)

```bash
gh workflow run "Publish Rust Crates" \
  --ref main \
  -f publish-kora-lib=true \
  -f publish-kora-cli=true \
  -f create-github-release=true
```

Monitor progress:

```bash
gh run list --workflow "Publish Rust Crates" --limit 3
```

---

## Step 6 — Trigger TypeScript publish workflow (if TS changed)

```bash
gh workflow run "Publish TypeScript SDK" \
  --ref main \
  -f publish-to-npm=true \
  -f create-github-release=true
```

Monitor progress:

```bash
gh run list --workflow "Publish TypeScript SDK" --limit 3
```

---

## Step 7 — Verify

```bash
# Rust — confirm tags and crates.io
git tag --sort=-version:refname | head -5

# TypeScript — confirm npm
npm view @solana/kora version
```

---

## Notes

- This skill is for mainline releases. For hotfix releases, trigger publish workflows from `hotfix/*` before merging back to `main`.
- Publish workflows allow `main` and `hotfix/*` refs.
- Rust publish order: `kora-lib` first, then `kora-cli` (kora-cli depends on kora-lib).
- A 30-second crates.io indexing delay is built into the CI workflow between the two publishes.
- TS prerelease versions are published with the `beta` npm tag; stable versions use `latest`.
- If a workflow fails mid-run, re-trigger with only the failed steps by setting the relevant booleans to `false`.
