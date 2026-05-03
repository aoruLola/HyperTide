param(
    [string]$ComposeFile = "$PSScriptRoot\docker-compose.prod.yml",
    [string]$EnvFile = "$PSScriptRoot\.env.production",
    [string]$BackupRoot = "$PSScriptRoot\backups"
)

$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $EnvFile)) {
    throw "Missing $EnvFile. Create it from .env.production.example first."
}

Get-Content -LiteralPath $EnvFile | ForEach-Object {
    if ($_ -match '^\s*#' -or $_ -notmatch '=') { return }
    $name, $value = $_ -split '=', 2
    [Environment]::SetEnvironmentVariable($name.Trim(), $value.Trim(), 'Process')
}

$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
$backupDir = Join-Path $BackupRoot $stamp
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

$dbDump = Join-Path $backupDir "postgres.sql"
docker compose -f $ComposeFile --env-file $EnvFile exec -T postgres pg_dump -U $env:POSTGRES_USER $env:POSTGRES_DB | Set-Content -LiteralPath $dbDump

$storageDir = Join-Path $PSScriptRoot "data\storage"
New-Item -ItemType Directory -Force -Path $storageDir | Out-Null
tar -czf (Join-Path $backupDir "storage.tar.gz") -C $storageDir .

$keysDir = Join-Path $PSScriptRoot "keys"
if (Test-Path -LiteralPath $keysDir) {
    tar -czf (Join-Path $backupDir "keys.tar.gz") -C $keysDir .
}

$manifest = @{
    created_at = $stamp
    compose_file = (Split-Path -Leaf $ComposeFile)
    postgres_db = $env:POSTGRES_DB
    storage_path = "deploy/server/data/storage"
    includes = @("postgres.sql", "storage.tar.gz", "keys.tar.gz")
} | ConvertTo-Json -Depth 3
$manifest | Set-Content -LiteralPath (Join-Path $backupDir "manifest.json")

Get-ChildItem -LiteralPath $backupDir -File | ForEach-Object {
    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName
    "$($hash.Hash.ToLowerInvariant())  $($_.Name)"
} | Set-Content -LiteralPath (Join-Path $backupDir "SHA256SUMS")

Write-Host "Backup written to $backupDir"
