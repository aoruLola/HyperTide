# HyperTide Backend Compose Deployment

## Quick Start

1. Optional: copy env template
   - `cp deploy/.env.example deploy/.env`
2. Start stack
   - `docker compose -f deploy/docker-compose.yml --env-file deploy/.env up -d --build`
   - If you did not create `deploy/.env`, use `deploy/.env.example` as `--env-file`.
3. Run smoke check
   - `pwsh ./deploy/smoke.ps1`

## What This Starts

- `postgres` (PostgreSQL 15)
- `jwt-keys` one-shot key generator (writes `deploy/keys/jwt-*.pem`)
- `hypertide` backend service (port `3000`)

## Notes

- `JWT_*_PATH` in container points to `/keys/jwt-private.pem` and `/keys/jwt-public.pem`.
- Storage persists to `../storage` from compose file location.
- For production, replace default passwords, pepper, and generated JWT keys.
