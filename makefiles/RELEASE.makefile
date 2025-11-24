# Release management targets

.PHONY: release

release:
	@echo "🚀 Release Process"
	@echo "=================="
	@echo ""
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "❌ Error: Working directory is not clean. Commit or stash changes first."; \
		exit 1; \
	fi
	@if ! command -v cargo-set-version >/dev/null 2>&1; then \
		echo "❌ Error: cargo-set-version not installed. Install with: cargo install cargo-edit"; \
		exit 1; \
	fi
	@if ! command -v git-cliff >/dev/null 2>&1; then \
		echo "❌ Error: git-cliff not installed. Install with: cargo install git-cliff"; \
		exit 1; \
	fi
	@echo "Current version: $$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "kora-lib") | .version')"
	@read -p "Enter new version (e.g., 2.0.0): " VERSION; \
	if [ -z "$$VERSION" ]; then \
		echo "❌ Error: Version cannot be empty"; \
		exit 1; \
	fi; \
	echo ""; \
	echo "📝 Updating version to $$VERSION..."; \
	cargo set-version --workspace $$VERSION; \
	echo ""; \
	echo "📋 Generating CHANGELOG.md..."; \
	LAST_TAG=$$(git tag -l "v*" --sort=-version:refname | head -1); \
	if [ -z "$$LAST_TAG" ]; then \
		git-cliff $$(git rev-list --max-parents=0 HEAD)..HEAD --config .github/cliff.toml --output CHANGELOG.md --strip all; \
	else \
		git-cliff $$LAST_TAG..HEAD --config .github/cliff.toml --output CHANGELOG.md --strip all; \
	fi; \
	echo ""; \
	echo "📦 Committing changes..."; \
	git add Cargo.toml Cargo.lock CHANGELOG.md crates/*/Cargo.toml; \
	git commit -m "chore: release v$$VERSION"; \
	echo ""; \
	echo "✅ Release prepared!"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  1. Push branch: git push origin HEAD"; \
	echo "  2. Create PR and merge to main"; \
	echo "  3. After merge, go to GitHub Actions and manually trigger 'Publish Rust Crates' workflow"
