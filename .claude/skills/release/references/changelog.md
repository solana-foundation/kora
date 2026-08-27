# CHANGELOG generation

git-cliff renders only the commit range you hand it, so an existing `CHANGELOG.md` has to be
concatenated underneath the new section rather than overwritten. Three cases:

```bash
last_tag=$(git tag -l "v*" --sort=-version:refname | head -1)

if [ -z "$last_tag" ]; then
  # First ever release: whole history
  git cliff "$(git rev-list --max-parents=0 HEAD)"..HEAD \
    --tag "v${RUST_VERSION}" --config .github/cliff.toml --output CHANGELOG.md --strip all

elif [ -f CHANGELOG.md ]; then
  # Normal case: prepend the new section to the existing file
  git cliff "${last_tag}"..HEAD \
    --tag "v${RUST_VERSION}" --config .github/cliff.toml --strip all > CHANGELOG.new.md
  cat CHANGELOG.md >> CHANGELOG.new.md
  mv CHANGELOG.new.md CHANGELOG.md

else
  git cliff "${last_tag}"..HEAD \
    --tag "v${RUST_VERSION}" --config .github/cliff.toml --output CHANGELOG.md --strip all
fi
```

`--output` truncates. Using it in the middle case silently drops every prior release, which is why
that branch redirects to a temp file instead.

`--strip all` removes the header/footer so the appended section stacks cleanly.

Grouping is driven by commit type via `.github/cliff.toml`: `feat`, `fix`, `perf`, `refactor`,
`doc`, `test` appear; `chore`, `ci`, `build` are skipped. A release with only skipped types
produces an empty section.

## Tools

`cargo install cargo-edit git-cliff` provides `cargo set-version` and `git cliff`. `gh` and `jq`
are also required.
