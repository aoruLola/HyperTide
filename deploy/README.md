# HyperTide Deployment

Deployment is now split by deliverable:

- [server](./server/README.md): backend container deployment
- [cli](./cli/README.md): `ht` packaging for internal distribution

## Recommended entrypoints

### Server

```powershell
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d --build
powershell -ExecutionPolicy Bypass -File .\deploy\server\smoke.ps1
```

### CLI

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\cli\package.ps1
```

```bash
bash ./deploy/cli/package.sh
```

## Compatibility note

The repository still contains the older top-level `deploy/Dockerfile`, `deploy/docker-compose.yml`, and `deploy/smoke.ps1` assets for continuity, but new deployments should prefer the split `server/` and `cli/` entrypoints.
