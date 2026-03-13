---
name: cherry-pick-sync
description: >-
  Sync commits from main to the active release/* branch via cherry-pick PR.
  Use when the user says "sync main to release", "cherry-pick to release",
  "sync release branch", "backport to release", "keep release up to date",
  or wants to ensure hotfixes/patches on main are reflected in the release branch.
  Also use when the user asks to "cherry-pick commits" between branches.
---

# Cherry-Pick Sync: Main to Release Branch

Cherry-pick commits from `main` into the active `release/*` branch and create a PR.
Resolve conflicts automatically when safe; ask the user when ambiguous.

## Workflow

### 1. Identify the release branch

```bash
# Prefer: open PRs from release/* branches (shows active work)
gh pr list --state open --head "release/" --json headRefName --jq '.[].headRefName' 2>/dev/null

# Fallback: most recent remote release branch by commit date
git branch -r --sort=-committerdate | grep 'origin/release/' | head -5
```

- One active `release/*` branch → use it.
- Multiple → ask the user.
- User specified one → use that.

### 2. Determine commits to cherry-pick

```bash
git fetch origin main <release-branch>
git log --oneline --reverse origin/<release-branch>..origin/main
```

**Filtering rules:**
- User specified commits (SHAs/keywords) → use only those.
- User said to skip certain commits → exclude them.
- Default → take ALL commits from the log above.
- **Auto-skip** release-only commits: version bumps, CHANGELOG updates, "chore: release v*". Flag these as skipped with reason.
- Show the final commit list to the user and confirm before proceeding.

### 3. Check preconditions

```bash
# Must have clean working directory
git status --porcelain
```

If dirty, ask the user to commit or stash first.

### 4. Create working branch

```bash
git checkout -b chore/cherry-pick-main-into-<release>-YYYYMMDD origin/<release-branch>
```

Use today's date (e.g., `chore/cherry-pick-main-into-release-2.2.0-20260313`).

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

**Already applied:** If cherry-pick says "already applied" or produces an empty commit, skip silently.

### 6. Push and create PR

```bash
git push -u origin HEAD

gh pr create \
  --base <release-branch> \
  --title "chore: sync main into <release-branch> (YYYY-MM-DD)" \
  --body "<see template below>"
```

**PR body template:**

```markdown
## Summary

Cherry-pick commits from `main` into `<release-branch>` to keep the release branch in sync with hotfixes and patches.

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
- **User on wrong branch** → always create working branch from `origin/<release-branch>` regardless of current branch.
- **Multiple release branches** → ask which one. Default to most recently committed.
