# Troubleshooting

When something fails, this section helps you move from symptom to root cause with minimal drama.

## Start Here

1. [Common Issues](./common-issues.md)
2. [Diagnostics Checklist](./diagnostics-checklist.md)

## Incident Response Pattern

1. Reproduce with exact command.
2. Capture output fully.
3. Classify failure domain.
4. Apply targeted fix.
5. Re-run full gate chain.
6. Record evidence path.

## Failure Domains

- Environment/toolchain
- Build/type errors
- Test regressions
- Governance policy failures
- Runtime behavior mismatches
- Integration compatibility drift

## Rule of Thumb

If your incident report does not include command, exit code, and artifact path, you have a story, not a diagnosis.
