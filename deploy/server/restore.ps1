param(
    [Parameter(Mandatory = $true)][string]$BackupDir,
    [string]$ComposeFile = "$PSScriptRoot\docker-compose.prod.yml",
    [string]$EnvFile = "$PSScriptRoot\.env.production"
)

$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $BackupDir)) {
    throw "Backup directory does not exist: $BackupDir"
}
if (!(Test-Path -LiteralPath $EnvFile)) {
    throw "Missing $EnvFile. Create it from .env.production.example first."
}

Get-Content -LiteralPath $EnvFile | ForEach-Object {
    if ($_ -match '^\s*#' -or $_ -notmatch '=') { return }
    $name, $value = $_ -split '=', 2
    [Environment]::SetEnvironmentVariable($name.Trim(), $value.Trim(), 'Process')
}

$storageDir = Join-Path $PSScriptRoot "data\storage"
New-Item -ItemType Directory -Force -Path $storageDir | Out-Null
if ((Get-ChildItem -LiteralPath $storageDir -Force | Select-Object -First 1)) {
    throw "Refusing to restore storage into a non-empty directory: $storageDir"
}

$tableCount = docker compose -f $ComposeFile --env-file $EnvFile exec -T postgres psql -U $env:POSTGRES_USER -d $env:POSTGRES_DB -tAc "select count(*) from information_schema.tables where table_schema = 'public';"
if ($tableCount.Trim() -ne "0" -and $env:RESTORE_ALLOW_NON_EMPTY_DB -ne "true") {
    throw "Refusing to restore into a database that already has public tables. Set RESTORE_ALLOW_NON_EMPTY_DB=true only after review."
}

tar -xzf (Join-Path $BackupDir "storage.tar.gz") -C $storageDir
$keysArchive = Join-Path $BackupDir "keys.tar.gz"
if (Test-Path -LiteralPath $keysArchive) {
    $keysDir = Join-Path $PSScriptRoot "keys"
    New-Item -ItemType Directory -Force -Path $keysDir | Out-Null
    tar -xzf $keysArchive -C $keysDir
}

Get-Content -LiteralPath (Join-Path $BackupDir "postgres.sql") | docker compose -f $ComposeFile --env-file $EnvFile exec -T postgres psql -U $env:POSTGRES_USER -d $env:POSTGRES_DB

Write-Host "Restore completed from $BackupDir"
