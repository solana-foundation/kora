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
		git-cliff $$(git rev-list --max-parents=0 HEAD)..HEAD --tag v$$VERSION --config .github/cliff.toml --output CHANGELOG.md --strip all; \
	else \
		if [ -f CHANGELOG.md ]; then \
			git-cliff $$LAST_TAG..HEAD --tag v$$VERSION --config .github/cliff.toml --strip all > CHANGELOG.new.md; \
			cat CHANGELOG.md >> CHANGELOG.new.md; \
			mv CHANGELOG.new.md CHANGELOG.md; \
		else \
			git-cliff $$LAST_TAG..HEAD --tag v$$VERSION --config .github/cliff.toml --output CHANGELOG.md --strip all; \
		fi; \
	fi; \
	echo ""; \
	echo "📦 Staging changes..."; \
	git add Cargo.toml Cargo.lock CHANGELOG.md crates/*/Cargo.toml; \
	echo ""; \
	echo "✅ Release prepared!"; \
	echo ""; \
	echo "Next steps:"; \
	echo "  1. Review CHANGELOG.md"; \
	echo "  2. Commit: git commit -m 'chore: release v$$VERSION'"; \
	echo "  3. Push branch: git push origin HEAD"; \
	echo "  4. Create PR and merge to main"; \
	echo "  5. After merge, go to GitHub Actions and manually trigger 'Publish Rust Crates' workflow"
