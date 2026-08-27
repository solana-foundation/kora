# Contributing

Kora is developed in public and we appreciate contributions.

## Before you start

- Search existing issues and pull requests before opening a new one.
- For substantial changes, open an issue or start a discussion first so maintainers can confirm the approach. Small PRs are preferred.
- Do not include secrets, private keys, seed phrases, or production credentials in issues, pull requests, commits, logs, or screenshots. Kora config references secrets by environment variable name (`*_env` fields in `signers.toml`); never inline a credential, even in an example.
- All commits into a Solana Foundation repository require [commit signature verification](https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification) to be enabled. Your PRs will not be merged without this.

## Branch targeting

The `main` branch is the integration branch. All feature work and bug fixes target `main` from a topic branch (`feat/*`, `fix/*`, `chore/*`, `hotfix/*`); CI rejects PRs opened from the deprecated `release/*` flow.

Audit status is commit-based, not branch-based. Audited baselines are tracked in [`audits/AUDIT_STATUS.md`](./audits/AUDIT_STATUS.md), and stable releases are represented by immutable tags/releases.

Do not use long-lived release branches. Use tags/releases plus [`audits/AUDIT_STATUS.md`](./audits/AUDIT_STATUS.md) to communicate audited baselines. Hotfixes branch from the deployed stable tag as `hotfix/*` and are merged back to `main` after publishing.

## Security vulnerabilities

Do not report security vulnerabilities in public issues. Follow the [security policy](./SECURITY.md).

## Development setup

Install Rust, Cargo, and [`just`](https://github.com/casey/just). The Rust toolchain is pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) and will be selected automatically.

```sh
just build            # build all workspace crates
just check            # formatting + clippy + TS type-check
just unit-test        # Rust unit tests
just integration-test # full integration suite (starts a local validator)
```

TypeScript SDK:

```sh
just install-ts-sdk
just build-ts-sdk
just unit-test-ts
```

Install the pre-commit hooks ([`.pre-commit-config.yaml`](./.pre-commit-config.yaml)) — they run `just fmt`, the TS formatter, and conventional-commit validation. Never bypass them with `--no-verify`; fix the cause instead.

Use the toolchain versions checked into the repository. Do not update the Rust toolchain, the Solana CLI, or package-manager versions as an incidental part of another change.

## Making a change

Keep changes focused. A pull request should solve one problem and include the tests, documentation, generated artifacts, or migration notes needed to keep the repository usable.

Before opening a pull request:

- Run `just check` and `just unit-test`. Run `just integration-test` when you touch validation, signing, fees, or the RPC surface.
- Add or update tests when behavior changes.
- Update documentation and reference material when the change is part of the user-facing contract — including the operator and client guides under [`.claude/skills/`](./.claude/skills/), which are documentation and drift like any other.
- Regenerate derived files with the repository's tooling rather than editing them by hand: `just gen-ts-client` for the SDK client, and the OpenAPI document it depends on.
- Explain any new dependency and why the existing dependency set is insufficient.

Kora is a paymaster: it signs transactions with a funded fee payer that it does not control the contents of. Any change touching transaction validation, the fee payer policy, fee calculation, or signer handling sits on a trust boundary. Document what an attacker could submit and why the change does not let them move fee payer funds. New fee payer policy flags must default to `false` and carry drain-safety property tests in `crates/lib/src/validator/transaction_validator/fee_payer_policy_props/`.

## Pull requests

Write a clear title and description that explain the problem, the approach, and how you tested it. Link related issues and call out behavior changes, compatibility concerns, or follow-up work. See [AI use](#ai-use) for how to disclose AI use in your PRs.

Use [Conventional Commits](https://www.conventionalcommits.org/) for commit and PR titles — the CHANGELOG is generated from them, and the type determines the version bump (`feat:` minor, `fix:` patch, `BREAKING CHANGE:` major).

By default, [Greptile](https://www.greptile.com) is enabled on all Solana Foundation repositories. Before maintainers review, all Greptile comments must be resolved with either a code fix or an explanation of why no change is needed.

Once CI is approved to run by maintainers, all CI errors must be addressed before the PR will be merged.

Maintainers may ask you to rebase, split a broad change, add tests, or revise documentation before merging.

## AI use

You may use AI-assisted tools, but you should review the generated code, understand its behavior, and run the same checks expected of any other contribution.

If you are building with AI on Solana, check out the [Solana Dev Skill](https://github.com/solana-foundation/solana-dev-skill) or the [Solana MCP](https://mcp.solana.com/) to aid in your work. This repository also ships [`CLAUDE.md`](./CLAUDE.md) and skills under [`.claude/skills/`](./.claude/skills/) that carry the repo's conventions and gotchas — point your tooling at them rather than rediscovering the codebase from scratch.

Ensure that the generated code adheres to the project's coding standards and best practices. Maintainers can close PRs if they appear to be low-effort AI slop. In particular, audit your changes for the following AI code smells that increase maintenance burden:

- Comments that explain why the _previous_ behavior was wrong and the new behavior is correct. This can be helpful context for reviewers as a GitHub comment in the review, but we do not need a history of every code change living in the codebase.
- Large blocks of comments with high density of technical jargon; comments should be distilled to clearly explain _why_ this code is doing something (if it's not obvious), not _what_ (the code should speak for itself).
- Drive-by refactoring of code that is not relevant to the actual change being made.

Be especially careful with AI-generated tests here. A test that asserts a validator rejects something is worthless if it would pass against a validator that rejects everything — assert the allowed case too, and make sure the test would actually fail if the policy flag it covers were flipped.

### Disclosure

It can be helpful to note the extent to which AI was used in the change. For example, adding

> I wrote all of the code for this feature, and had Claude update the documentation and create tests accordingly

or

> I architected the change and handed all implementation over to Codex

to the pull request description can be helpful context for reviewers.

### Communication

If maintainers have suggested changes, feedback, or questions about your code, you should not be copy/pasting the questions to an LLM and copy/pasting the response. You being able to distill the information that AI produces is what makes your contribution valuable.

## License

By contributing, you agree that your contributions are licensed under the project's [LICENSE](./LICENSE.md).
