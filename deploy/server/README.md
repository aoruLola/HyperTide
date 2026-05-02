# HyperTide Server Deployment

This directory contains the server-only deployment entrypoints for HyperTide.

## What lives here

- `Dockerfile`: backend image build for `hypertide-server`
- `docker-compose.yml`: PostgreSQL + JWT key bootstrap + backend service
- `.env.example`: server deployment environment template
- `smoke.ps1`: wrapper for the existing runtime smoke workflow

## Quick Start

```powershell
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d --build
powershell -ExecutionPolicy Bypass -File .\deploy\server\smoke.ps1
```

## What this deploys

- `postgres`
- `jwt-keys`
- `hypertide` backend service

## Notes

- This is the preferred server deployment entrypoint going forward.
- The backend image is built from `deploy/server/Dockerfile`.
- JWT keys are generated into `deploy/server/keys/`.
- Persistent asset storage remains at the repository-level `storage/` directory.
- For production, replace the example database password, pepper, and generated JWT keys.
