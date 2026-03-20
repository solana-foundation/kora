---
name: release
description: "Prepare a Kora release PR. Bumps Rust crate versions (kora-lib + kora-cli), optionally bumps the TypeScript SDK, generates CHANGELOG, and opens a PR against main. Use when the user says 'prepare a release', 'cut a release', or 'release version X.Y.Z'."
---

# Kora Release Preparation

Prepare a release PR for Kora. This updates version numbers, generates the CHANGELOG, and opens a PR against `main`. No publishing happens here — CI workflows handle that after merge.

---

## Prerequisites

Verify these tools are installed before starting:

```bash
cargo set-version --version   # from cargo-edit: cargo install cargo-edit
git cliff --version           # cargo install git-cliff
gh --version                  # GitHub CLI
jq --version
```

---

## Step 1 — Get current versions

```bash
# Rust (workspace root Cargo.toml)
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "kora-lib") | .version'

# TypeScript SDK
node -p "require('./sdks/ts/package.json').version"
```

Ask the user which component(s) to release and what version(s) to use.

---

## Step 2 — Verify clean working directory

```bash
git status --porcelain
```

If output is non-empty, stop and ask the user to commit or stash first.

---

## Step 3 — Create release branch

```bash
# Rust-only release
git checkout -b "chore/release-v${RUST_VERSION}"

# Both Rust + TypeScript
git checkout -b "chore/release-v${RUST_VERSION}-ts-v${TS_VERSION}"
```

---

## Step 4 — Bump Rust versions

```bash
# Update all workspace crates to the new version
cargo set-version --workspace "${RUST_VERSION}"

# Update the kora-lib version pin in workspace.dependencies
sed -i.bak \
  "s/kora-lib = { path = \"crates\/lib\", version = \"[^\"]*\" }/kora-lib = { path = \"crates\/lib\", version = \"${RUST_VERSION}\" }/" \
  Cargo.toml
rm -f Cargo.toml.bak
```

---

## Step 5 — Generate CHANGELOG

```bash
last_tag=$(git tag -l "v*" --sort=-version:refname | head -1)

if [ -z "$last_tag" ]; then
  git cliff "$(git rev-list --max-parents=0 HEAD)"..HEAD \
    --tag "v${RUST_VERSION}" \
    --config .github/cliff.toml \
    --output CHANGELOG.md \
    --strip all
elif [ -f CHANGELOG.md ]; then
  git cliff "${last_tag}"..HEAD \
    --tag "v${RUST_VERSION}" \
    --config .github/cliff.toml \
    --strip all > CHANGELOG.new.md
  cat CHANGELOG.md >> CHANGELOG.new.md
  mv CHANGELOG.new.md CHANGELOG.md
else
  git cliff "${last_tag}"..HEAD \
    --tag "v${RUST_VERSION}" \
    --config .github/cliff.toml \
    --output CHANGELOG.md \
    --strip all
fi
```

---

## Step 6 — Bump TypeScript SDK (if releasing TS)

```bash
npm version "${TS_VERSION}" --no-git-tag-version --prefix sdks/ts
```

---

## Step 7 — Stage and commit

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md crates/*/Cargo.toml

# If TS was bumped
git add sdks/ts/package.json

# Rust-only commit message
git commit -m "chore: release v${RUST_VERSION}"

# Combined commit message
git commit -m "chore: release v${RUST_VERSION} rust + ts-sdk v${TS_VERSION}"
```

---

## Step 8 — Push and open PR

```bash
git push -u origin HEAD

gh pr create \
  --base main \
  --title "chore: release v${RUST_VERSION}" \
  --reviewer dev-jodee,amilz \
  --body "$(cat <<EOF
## Release v${RUST_VERSION}

### Rust Crates
- **kora-lib** \`${RUST_VERSION}\`
- **kora-cli** \`${RUST_VERSION}\`
- CHANGELOG updated from commits since last tag

### TypeScript SDK
- **@solana/kora** \`${TS_VERSION}\` *(omit section if not releasing)*

## Post-merge
Trigger CI workflows from \`main\` using the \`complete-release\` skill (or manually):
- **Rust**: Actions → "Publish Rust Crates"
- **TypeScript**: Actions → "Publish TypeScript SDK"
EOF
)"
```

---

## Notes

- All release PRs target `main` regardless of current branch.
- Do NOT call `just release` or `just release-ts-sdk` — both are interactive.
- Tags (`v{VERSION}`, `kora-lib-v{VERSION}`, `kora-cli-v{VERSION}`) are created by CI after merge.
- Prerelease versions use semver suffixes, e.g. `2.3.0-beta.1`.
