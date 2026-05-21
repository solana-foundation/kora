# Contributing

Kora is developed in public and we appreciate contributions.

## Important: Branch Targeting

The `main` branch is the integration branch. All feature work and bug fixes should target `main`.

Audit status is commit-based, not branch-based. Audited baselines are tracked in [`audits/AUDIT_STATUS.md`](./audits/AUDIT_STATUS.md), and stable releases are represented by immutable tags/releases.

Do not use long-lived release branches. Use tags/releases plus [`audits/AUDIT_STATUS.md`](./audits/AUDIT_STATUS.md) to communicate audited baselines.

## Security

For security vulnerabilities related to code on `main`, please review [SECURITY.md](./SECURITY.md) before opening a public issue.

## Getting Started

1. Install Rust, Cargo, and `just`
2. Build all packages: `just build`
3. Run formatting and lint checks: `just check`
4. Run unit tests: `just unit-test`
5. Run integration tests: `just integration-test`

## TypeScript SDK

```shell
just install-ts-sdk
just build-ts-sdk
just unit-test-ts
```

## Before Submitting

- Run `just check` (formatting + clippy)
- Run `just unit-test`
- Use [conventional commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, etc.)
