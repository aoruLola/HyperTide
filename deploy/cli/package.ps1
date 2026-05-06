param(
    [string]$OutDir = "deploy/cli/dist"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
$outDirFull = Join-Path $root $OutDir

Push-Location $root
try {
    $version = cargo pkgid -p hypertide-cli
    if ($LASTEXITCODE -ne 0) {
        throw "failed to resolve hypertide-cli package version"
    }
    if ($version -match '@(?<version>[^#\s]+)$') {
        $version = $Matches.version
    } else {
        throw "failed to parse hypertide-cli version from cargo pkgid output: $version"
    }

    cargo build -p hypertide-cli --bin ht --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed"
    }

    New-Item -ItemType Directory -Force -Path $outDirFull | Out-Null

    $binary = Join-Path $root "target\\release\\ht.exe"
    if (-not (Test-Path $binary)) {
        throw "missing built binary: $binary"
    }

    $staging = Join-Path $outDirFull "hypertide-cli-$version-windows-x86_64"
    $stagedBinary = Join-Path $staging "ht.exe"
    if (Test-Path $stagedBinary) {
        Remove-Item -Path $stagedBinary -Force
    }
    if (Test-Path $staging) {
        Remove-Item -Path $staging -Force
    }
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    Copy-Item -Path $binary -Destination $stagedBinary -Force

    $zipPath = Join-Path $outDirFull "hypertide-cli-$version-windows-x86_64.zip"
    if (Test-Path $zipPath) {
        Remove-Item -Path $zipPath -Force
    }
    Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $zipPath

    Write-Output "packaged CLI artifact: $zipPath"
}
finally {
    Pop-Location
}
