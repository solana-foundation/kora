# Audit Status

Last updated: 2026-03-18

## Current Baseline

- Auditor: Runtime Verification
- Report: `audits/20251119_runtime-verification.pdf`
- Audited-through commit: `8c592591debd08424a65cc471ce0403578fd5d5d`
- Compare unaudited delta: https://github.com/solana-foundation/kora/compare/8c592591debd08424a65cc471ce0403578fd5d5d...main

Audit scope is commit-based. Commits after the audited-through SHA are considered unaudited until a new audit or mitigation review updates this file.

## Branch and Release Model

- `main` is the integration branch and may contain audited and unaudited commits.
- Stable production releases are immutable tags/releases (for example `v2.3.0`).
- Audited baselines are tracked by commit SHA plus immutable tags/releases, not by long-lived release branches.

## Verification Commands

```bash
# Count commits after the audited baseline
git rev-list --count 8c592591debd08424a65cc471ce0403578fd5d5d..main

# Inspect commit list since audited baseline
git log --oneline 8c592591debd08424a65cc471ce0403578fd5d5d..main

# Inspect file-level diff since audited baseline
git diff --name-status 8c592591debd08424a65cc471ce0403578fd5d5d..main
```

## Maintenance Rules

When a new audit is completed:

1. Add the new report to `audits/`.
2. Update `Audited-through commit` and `Compare unaudited delta`.
3. Tag audited release commit(s) (for example `vX.Y.Z`).
4. Update README and release notes links if needed.
