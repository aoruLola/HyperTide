param(
    [string]$BaseUrl = "http://localhost:3000"
)

$ErrorActionPreference = "Stop"

function Assert-Success($Condition, $Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Invoke-Cli($Arguments, $WorkingDirectory) {
    Push-Location $WorkingDirectory
    try {
        & cargo run -p hypertide-cli --bin ht -- @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "ht command failed: $($Arguments -join ' ')"
        }
    }
    finally {
        Pop-Location
    }
}

Write-Host "Smoke start: $BaseUrl" -ForegroundColor Cyan

$live = Invoke-RestMethod -Method Get -Uri "$BaseUrl/health/live"
Assert-Success ($live -eq "OK") "health/live failed"

try {
    Invoke-RestMethod -Method Get -Uri "$BaseUrl/health/ready" | Out-Null
}
catch {
    throw "health/ready failed: $($_.Exception.Message)"
}

$exchangeBody = @{ api_key = "dev-master-key" } | ConvertTo-Json
$exchange = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v2/auth/exchange-key" -Body $exchangeBody -ContentType "application/json"
Assert-Success ($exchange.success -eq $true) "auth/exchange-key failed"
Assert-Success (-not [string]::IsNullOrWhiteSpace($exchange.data.access_token)) "missing access_token"

$verify = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v2/auth/verify" -Headers @{ "X-API-Key" = "dev-master-key" }
Assert-Success ($verify.success -eq $true) "auth/verify envelope failed"
Assert-Success ($verify.data.valid -eq $true) "auth/verify valid=false"

$repoRoot = Split-Path -Parent $PSScriptRoot
$smokeRunId = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
$workspace = Join-Path $repoRoot "tmp\deploy-smoke-$smokeRunId"
New-Item -ItemType Directory -Path $workspace -Force | Out-Null

$repoId = "smoke-repo-$smokeRunId"
$assetPath = "Content/Smoke/hello.txt"
$localFile = Join-Path $workspace "hello.txt"
Set-Content -Path $localFile -Value "hello from smoke $smokeRunId" -NoNewline

Invoke-Cli @(
    "login",
    "--server", $BaseUrl,
    "--token", "dev-master-key",
    "--api-key-direct",
    "--repo", $repoId,
    "--branch", "main"
) $workspace

Invoke-Cli @(
    "branch", "create",
    "--repo", $repoId,
    "--name", "smoke-bootstrap"
) $workspace

Invoke-Cli @(
    "add",
    "--file", $localFile,
    "--asset-path", $assetPath
) $workspace

Invoke-Cli @(
    "submit",
    "--message", "runtime smoke submit"
) $workspace

Invoke-Cli @("sync") $workspace
Invoke-Cli @("checkout") $workspace
Invoke-Cli @("status") $workspace
Invoke-Cli @("diff") $workspace

$checkedOutFile = Join-Path $workspace "Content\Smoke\hello.txt"
Assert-Success (Test-Path $checkedOutFile) "checkout did not materialize expected asset"

Write-Host "Smoke passed." -ForegroundColor Green
