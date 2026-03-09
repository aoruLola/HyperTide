# Auth Bootstrap Reference

Use this reference when the agent needs a short reminder of the HyperTide authentication stance.

## Preferred mode

- Default: JWT exchange via `ht login --server ... --token ...`
- Fallback: `--api-key-direct` only for development or emergency paths

## Required context

Always gather:

- server
- token
- repo
- branch

## Report back after login

After suggesting or running login, summarize:

- server host
- active repo
- active branch
- auth mode used: `jwt` or `api-key-direct`

Do not repeat the raw token value in the summary.
