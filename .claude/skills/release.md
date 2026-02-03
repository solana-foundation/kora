# Full Release Skill

## Overview
This skill automates the complete release process for both Rust crates and TypeScript SDK, handling the stash/unstash workflow and creating a PR.

## When to Use
Use this skill when the user wants to:
- Release both Rust and TypeScript packages together
- Create a release PR for either beta (release/* branch) or stable (main branch)

## Prerequisites
Before starting, verify these tools are installed:
- `cargo-set-version` (cargo install cargo-edit)
- `git-cliff` (cargo install git-cliff)
- `gh` CLI (for creating PRs)
- `jq` (for parsing JSON)

## Release Flow

### Step 1: Determine Release Type and Base Branch
Check the current branch to determine the release type:
```bash
current_branch=$(git branch --show-current)
```

- If on `main` → Stable release, base branch is `main`
- If on `release/*` → Beta release, base branch is that release branch (e.g., `release/2.2.0`)
- If on any other branch → Ask user which base branch to use

### Step 2: Detect Build System
Check which build system is available:
```bash
# Check for justfile first (preferred)
if [ -f "justfile" ] || [ -f "Justfile" ]; then
    BUILD_SYSTEM="just"
elif [ -f "Makefile" ]; then
    BUILD_SYSTEM="make"
else
    echo "No justfile or Makefile found"
    exit 1
fi
```

### Step 3: Verify Clean Working Directory
```bash
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Working directory not clean. Commit or stash changes first."
    exit 1
fi
```

### Step 4: Get Version Information
Prompt the user for version numbers:
1. **Rust version**: Ask user for the new Rust crate version
2. **TypeScript SDK version**: Ask user for the new TypeScript SDK version

Get current versions for reference:
```bash
# Current Rust version
cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "kora-lib") | .version'

# Current TypeScript SDK version
node -p "require('./sdks/ts/package.json').version"
```

### Step 5: Create Release Branch
Create a dedicated branch for the release changes:
```bash
release_branch="release-prep/v${RUST_VERSION}"
git checkout -b "$release_branch"
```

### Step 6: Run Rust Release (Non-Interactive)
Execute the Rust release steps manually (since `just release` is interactive):

```bash
# Update Rust versions
cargo set-version --workspace "$RUST_VERSION"

# Update kora-lib version in workspace.dependencies
sed -i.bak "s/kora-lib = { path = \"crates\/lib\", version = \"[^\"]*\" }/kora-lib = { path = \"crates\/lib\", version = \"$RUST_VERSION\" }/" Cargo.toml
rm -f Cargo.toml.bak

# Generate CHANGELOG
last_tag=$(git tag -l "v*" --sort=-version:refname | head -1)
if [ -z "$last_tag" ]; then
    git-cliff $(git rev-list --max-parents=0 HEAD)..HEAD --tag "v$RUST_VERSION" --config .github/cliff.toml --output CHANGELOG.md --strip all
elif [ -f CHANGELOG.md ]; then
    git-cliff "$last_tag"..HEAD --tag "v$RUST_VERSION" --config .github/cliff.toml --strip all > CHANGELOG.new.md
    cat CHANGELOG.md >> CHANGELOG.new.md
    mv CHANGELOG.new.md CHANGELOG.md
else
    git-cliff "$last_tag"..HEAD --tag "v$RUST_VERSION" --config .github/cliff.toml --output CHANGELOG.md --strip all
fi

# Stage Rust changes
git add Cargo.toml Cargo.lock CHANGELOG.md crates/*/Cargo.toml
```

### Step 7: Stash Rust Changes
```bash
git stash push -m "rust-release-v${RUST_VERSION}"
```

### Step 8: Run TypeScript SDK Release (Non-Interactive)
Execute the TypeScript SDK release steps manually:

```bash
# Update TypeScript SDK version
npm version "$TS_VERSION" --no-git-tag-version --prefix sdks/ts

# Stage TypeScript changes
git add sdks/ts/package.json
```

### Step 9: Unstash Rust Changes
```bash
git stash pop
```

### Step 10: Commit All Changes
```bash
git add -A
git commit -m "chore: release v${RUST_VERSION} (rust) + ts-sdk v${TS_VERSION}"
```

### Step 11: Push and Create PR
```bash
git push -u origin "$release_branch"

# Determine PR base branch
if [[ "$current_branch" == release/* ]]; then
    base_branch="$current_branch"
    pr_title="chore: beta release v${RUST_VERSION}"
else
    base_branch="main"
    pr_title="chore: release v${RUST_VERSION}"
fi

# Create PR
gh pr create \
    --base "$base_branch" \
    --title "$pr_title" \
    --body "$(cat <<EOF
## Release Summary

### Rust Crates
- **Version**: v${RUST_VERSION}
- Updates \`kora-lib\` and \`kora-cli\` versions
- CHANGELOG generated from commits since last release

### TypeScript SDK
- **Version**: v${TS_VERSION}
- Updates \`sdks/ts/package.json\`

## Post-Merge Steps

### Rust Crates
After merging, trigger the **Publish Rust Crates** workflow to:
- Create git tags (v${RUST_VERSION}, kora-lib-v${RUST_VERSION}, kora-cli-v${RUST_VERSION})
- Publish to crates.io

### TypeScript SDK
Trigger the **Publish TypeScript SDK (Manual)** workflow to publish to npm.
EOF
)"
```

## Important Notes

1. **Version Format**: Use semantic versioning (e.g., `2.2.0`, `2.2.0-beta.1`)
2. **Beta Releases**: When on a `release/*` branch, the PR targets that branch
3. **Stable Releases**: When on `main`, the PR targets `main`
4. **Changelog**: Only generated for Rust releases, not TypeScript SDK
5. **Tags**: Created by CI workflows after PR merge, not during release prep

## Error Handling

- If Rust release fails, clean up with: `git checkout -- . && git clean -fd`
- If stash fails, the Rust changes are already staged - proceed with TS release
- If PR creation fails, the branch is pushed - create PR manually via GitHub UI

## Makefile Equivalent Commands

If using Makefile instead of justfile, the commands would be:
- `make release` instead of `just release`
- `make release-ts-sdk` instead of `just release-ts-sdk`

The skill should detect and use the appropriate build system, but execute the release steps directly (non-interactively) rather than calling the interactive commands.
