---
name: release
description: "Prepare a Kora release PR. Bumps Rust crate versions (kora-lib + kora-cli), optionally bumps the TypeScript SDK, generates CHANGELOG, and opens a PR against main. Use when the user says 'prepare a release', 'cut a release', or 'release version X.Y.Z'."
---

# Kora Release Preparation

Bump versions, generate the CHANGELOG, open a PR against `main`. Publishing happens after merge
via CI — see the `complete-release` skill.

## Decide first

Read current versions, then ask the user which components to release and at what versions:

```bash
cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="kora-lib") | .version'
node -p "require('./sdks/ts/package.json').version"
```

Rust and TypeScript release independently. Prereleases use semver suffixes (`2.3.0-beta.1`).

Work on a `chore/release-v${VERSION}` branch off `main` with a clean tree.

## Bump only kora-lib and kora-cli

**Do not run `cargo set-version --workspace`.** The workspace also contains `crates/kora-deploy`
(published separately, currently on its own `0.x` line) and `examples/devnet-deploy-paymaster`.
A workspace-wide bump drags both onto the Kora version and `kora-deploy` has no `publish = false`
to stop it reaching crates.io.

Four places move together, and `kora-lib` + `kora-cli` always share one version:

```bash
cargo set-version -p kora-lib -p kora-cli "${RUST_VERSION}"
```

Then edit by hand in the root `Cargo.toml`:
- `[workspace.package] version` — `tests` inherits this
- the `kora-lib = { path = "crates/lib", version = "..." }` pin in `[workspace.dependencies]`

Confirm nothing else moved before committing:

```bash
git diff --stat
grep -rn '^version' crates/*/Cargo.toml Cargo.toml
```

`crates/kora-deploy/Cargo.toml` must be untouched.

## CHANGELOG

git-cliff appends rather than regenerates, and the invocation differs depending on whether a
previous tag and CHANGELOG exist. See [references/changelog.md](references/changelog.md).

## TypeScript

```bash
npm version "${TS_VERSION}" --no-git-tag-version --prefix sdks/ts
```

## Commit and PR

Commit message: `chore: release v${RUST_VERSION}`, plus ` rust + ts-sdk v${TS_VERSION}` when both
move. PR targets `main`, reviewers `dev-jodee,amilz`. Body should list the crates and versions
being released and point at the `complete-release` skill for publishing.

## Constraints

- Release PRs always target `main`, regardless of the current branch.
- Hotfix patches publish from `hotfix/*` *before* merge-back to `main`.
- Never call `just release` or `just release-ts-sdk` — both are interactive and will hang.
- Tags (`v{VERSION}`, `kora-lib-v{VERSION}`, `kora-cli-v{VERSION}`) are created by CI after merge,
  not here.
