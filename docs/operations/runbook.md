# HyperTide Operations Runbook

This runbook covers the Docker Compose self-hosted deployment.

## Startup Fails

Check service status:

```bash
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production ps
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production logs hypertide
```

Common causes:

- `APP_ENV=production` with a missing secret.
- A `CHANGE_ME` placeholder still present.
- `CORS_ALLOWED_ORIGINS` missing.
- `keys/jwt-private.pem`, `keys/jwt-public.pem`, or `keys/witness-config.json` missing.
- `data/storage` not writable by the server container user.

## DB Unavailable

Symptoms:

- `/health/ready` returns `DB_UNAVAILABLE` or `DB_NOT_CONFIGURED`.
- Server logs include Postgres connection or migration errors.

Actions:

```bash
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production ps postgres
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production logs postgres
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production exec postgres pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"
```

Verify `DATABASE_URL` matches the Compose database name, user, password, and host `postgres`.

## Storage Unavailable

Symptoms:

- `/health/ready` returns `STORAGE_UNAVAILABLE`.
- Uploads fail with storage persistence errors.

Actions:

```bash
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production exec hypertide sh -lc 'ls -la /app/storage /app/storage/objects /app/storage/temp'
docker compose -f deploy/server/docker-compose.prod.yml --env-file deploy/server/.env.production logs hypertide
```

Check host disk space and permissions for `deploy/server/data/storage`.

## Migration Failure

Symptoms:

- Server exits during startup after database pool initialization.
- Logs mention migration failure.

Actions:

1. Stop the server container, leaving Postgres running.
2. Confirm the release notes for migration requirements.
3. Restore the backup taken before the upgrade if the migration partially changed schema.
4. Re-run the previous image tag until the target migration is fixed.

Every release that changes schema must document migration impact and rollback expectations.

## Disk Space Low

Symptoms:

- Postgres writes fail.
- Storage probe fails.
- Backups fail or produce partial artifacts.

Actions:

```bash
df -h
du -sh deploy/server/data/postgres deploy/server/data/storage deploy/server/backups
```

Move old verified backups to durable external storage. Do not remove production data directories as an emergency shortcut.

## Restore Backup

Use a fresh target whenever possible:

```bash
cd deploy/server
./restore.sh ./backups/20260503T120000Z
./smoke.sh
```

The restore scripts refuse non-empty storage directories and non-empty DB schemas by default. That guard is intentional. If a restore needs to replace live data, prepare a separate reviewed maintenance procedure.

## Roll Back Version

When no schema downgrade is needed:

```bash
cd deploy/server
HYPERTIDE_VERSION=v0.1.0 docker compose -f docker-compose.prod.yml --env-file .env.production up -d
./smoke.sh
```

When schema or data changed, restore the backup taken immediately before the upgrade, then start the previous image tag.

## API Key Revoke

Use the CLI or API key management endpoint to revoke compromised keys. After revocation:

- Confirm the key no longer authenticates.
- Review audit entries for actions performed by that actor.
- Rotate related local credentials when the key was stored in an exposed environment.

## Key Rotation

Rotate one secret class at a time:

- `MASTER_KEY`: schedule downtime unless all clients can re-authenticate safely.
- `AUTH_PEPPER`: plan API key re-issue or verification impact.
- JWT signing keys: deploy new key pair and invalidate old refresh tokens if needed.
- Witness secrets: update quorum config and verify witness topology before resuming high-risk workflows.

Always take a backup before rotation.
