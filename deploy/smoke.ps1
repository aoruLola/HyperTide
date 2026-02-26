param(
    [string]$BaseUrl = "http://localhost:3000"
)

$ErrorActionPreference = "Stop"

function Assert-Success($Condition, $Message) {
    if (-not $Condition) {
        throw $Message
    }
}

Write-Host "Smoke start: $BaseUrl" -ForegroundColor Cyan

$live = Invoke-RestMethod -Method Get -Uri "$BaseUrl/health/live"
Assert-Success ($live -eq "OK") "health/live failed"

$ready = Invoke-WebRequest -Method Get -Uri "$BaseUrl/health/ready"
Assert-Success ($ready.StatusCode -eq 200) "health/ready failed"

$exchangeBody = @{ api_key = "dev-master-key" } | ConvertTo-Json
$exchange = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v2/auth/exchange-key" -Body $exchangeBody -ContentType "application/json"
Assert-Success ($exchange.success -eq $true) "auth/exchange-key failed"
Assert-Success (-not [string]::IsNullOrWhiteSpace($exchange.data.access_token)) "missing access_token"

$verify = Invoke-RestMethod -Method Get -Uri "$BaseUrl/v2/auth/verify" -Headers @{ "X-API-Key" = "dev-master-key" }
Assert-Success ($verify.success -eq $true) "auth/verify envelope failed"
Assert-Success ($verify.data.valid -eq $true) "auth/verify valid=false"

Write-Host "Smoke passed." -ForegroundColor Green
