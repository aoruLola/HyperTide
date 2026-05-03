param(
    [string]$BaseUrl = "http://127.0.0.1:3000",
    [int]$Attempts = 30,
    [int]$SleepSeconds = 2
)

$ErrorActionPreference = "Stop"

function Wait-ForUrl {
    param([string]$Path)
    for ($i = 1; $i -le $Attempts; $i++) {
        try {
            Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl$Path" | Out-Null
            return
        } catch {
            Start-Sleep -Seconds $SleepSeconds
        }
    }
    throw "Timed out waiting for $BaseUrl$Path"
}

Wait-ForUrl -Path "/health/live"
Wait-ForUrl -Path "/health/ready"
$metrics = Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/metrics"
if ($metrics.Content -notmatch "hypertide_http_requests_total") {
    throw "Metrics endpoint did not expose hypertide_http_requests_total"
}

Write-Host "HyperTide smoke checks passed for $BaseUrl"
