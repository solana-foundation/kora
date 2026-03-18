---
name: cherry-pick-sync
description: >-
  Deprecated for normal development. Only use this for short-lived stabilization
  branches (for example `release/*`) when explicitly requested by the user.
  Do not use this as a default branch strategy.
---

# Cherry-Pick Sync: Main to Stabilization Branch

## Status

This workflow is deprecated for normal development. `main` is the default integration branch.
Use this skill only for exceptional short-lived stabilization branches.

Cherry-pick commits from `main` into a user-specified stabilization branch and create a PR.
Resolve conflicts automatically when safe; ask the user when ambiguous.

## Workflow

### 1. Identify the stabilization branch

```bash
# If user did not specify a branch, inspect likely candidates
git branch -r --sort=-committerdate | grep 'origin/release/' | head -5
```

- User specified a branch → use that.
- If not specified, ask user which branch to target and suggest the most recently updated candidate.
- Do not assume long-lived `release/*` syncing is required.

### 2. Determine commits to cherry-pick

**Find the last sync point first.** Prior cherry-pick syncs are often squash-merged, so `git cherry` and `git log` cannot detect them as equivalent. Instead:

```bash
git fetch origin main <stabilization-branch>

# Find the most recent cherry-pick sync PR on the target branch
git log --oneline origin/<stabilization-branch> | grep -i -E "cherry.pick|sync.*main|bring back commits from main" | head -3
```

Read the PR body of the most recent sync (use `gh pr view <number>`) to identify which main commits it included. The **last main commit referenced** in that PR is your sync baseline.

Then list only commits **after** the sync baseline:

```bash
git log --oneline --reverse <last-synced-sha>..origin/main
```

If no prior sync PR exists, fall back to the full range:

```bash
git log --oneline --reverse origin/<stabilization-branch>..origin/main
```

**Filtering rules:**
- User specified commits (SHAs/keywords) → use only those.
- User said to skip certain commits → exclude them.
- Default → take ALL commits from the log above.
- **Auto-skip** release-only commits: version bumps, CHANGELOG updates, "chore: release v*". Flag these as skipped with reason.
- **Auto-skip** commits already on target: use `git cherry -v origin/<stabilization-branch> origin/main` as a secondary check — lines starting with `-` are already applied.
- Show the final commit list to the user and confirm before proceeding.

### 3. Check preconditions

```bash
# Must have clean working directory
git status --porcelain
```

If dirty, ask the user to commit or stash first.

### 4. Create working branch

Replace `/` with `-` in the release branch name when constructing the working branch:

```bash
git checkout -b chore/cherry-pick-main-into-$(echo <stabilization-branch> | tr '/' '-')-YYYYMMDD origin/<stabilization-branch>
```

Example: `release/2.2.0` → `chore/cherry-pick-main-into-release-2.2.0-20260313`.

### 5. Cherry-pick commits

Cherry-pick one at a time to isolate conflicts:

```bash
git cherry-pick <sha>
```

**On conflict:**
1. Inspect the conflict markers.
2. **Auto-resolve** if obvious and low-risk: import ordering, trivial merge context, whitespace.
3. **Ask the user** if: logic changes, behavioral differences, unclear intent, or risk of breaking something.
4. After resolving: `git add .` then `git cherry-pick --continue`.
5. If a commit cannot be applied and user agrees → `git cherry-pick --skip`.

Track all skipped and conflict-resolved commits for the PR body.

**Abort/rollback:** If the user wants to cancel the entire operation:
```bash
git cherry-pick --abort
git checkout -
git branch -D <working-branch>
```
Inform the user the repo has been restored to its original state.

**Already applied:** If cherry-pick says "already applied" or produces an empty commit, skip silently.

### 6. Push and create PR

```bash
git push -u origin HEAD

gh pr create \
  --base <stabilization-branch> \
  --title "chore: sync main into <stabilization-branch> (YYYY-MM-DD)" \
  --body "<see template below>"
```

**PR body template:**

```markdown
## Summary

Cherry-pick commits from `main` into `<stabilization-branch>` to keep the branch in sync with selected fixes.

### Commits included

- `abc1234` fix: description (#123)
- `def5678` feat: description (#456)

### Commits skipped

- `ghi9012` chore: release v2.0.5 (release-only)
- _None_ (if empty)

### Conflicts resolved

- `path/to/file.rs`: brief explanation of resolution
- _None_ (if empty)
```

### 7. Return the PR URL

## Edge Cases

- **No new commits** → tell user branches are in sync. Do not create empty PR.
- **User on wrong branch** → always create working branch from `origin/<stabilization-branch>` regardless of current branch.
- **Multiple candidate branches** → ask which one; suggest the most recently committed as default.
