param(
    [string]$BaseUrl = "http://localhost:3000"
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent (Split-Path -Parent $scriptDir)
$delegate = Join-Path $root "deploy\\smoke.ps1"

powershell -ExecutionPolicy Bypass -File $delegate -BaseUrl $BaseUrl
exit $LASTEXITCODE
