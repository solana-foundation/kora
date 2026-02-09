# Contributing

Kora is developed in public and we appreciate contributions.

## Important: Branch Targeting

The `main` branch only contains **audited releases** plus minor hotfixes and docs. All feature work and bug fixes should target the latest `release/*` branch (check [open PRs](https://github.com/solana-foundation/kora/pulls) or the [README](https://github.com/solana-foundation/kora#readme) for the current one).

PRs opened against `main` for non-audited code will be asked to rebase onto the active release branch.

## Security

For security vulnerabilities related to code on `main`, please review [SECURITY.md](./SECURITY.md) before opening a public issue.

## Getting Started

1. Install Rust and Cargo
2. Build all packages: `make build`
3. Run formatting and lint checks: `make check`
4. Run unit tests: `make unit-test`
5. Run integration tests: `make integration-test`

## TypeScript SDK

```shell
make install-ts-sdk
make build-ts-sdk
make unit-test-ts
```

## Before Submitting

- Run `make check` (formatting + clippy)
- Run `make unit-test`
- Use [conventional commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, etc.)
