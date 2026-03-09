param(
    [ValidateSet("codex", "agents", "both")]
    [string]$Target = "both",
    [string]$CodexHome = $(if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }),
    [string]$AgentsHome = $(Join-Path $HOME ".agents")
)

$ErrorActionPreference = "Stop"

$sourceRoot = $PSScriptRoot
$skillNames = @(
    "hypertide-auth-bootstrap",
    "hypertide-workspace-flow",
    "hypertide-versioning-ops",
    "hypertide-trust-audit"
)

$targetRoots = @()
switch ($Target) {
    "codex"  { $targetRoots += (Join-Path $CodexHome "skills") }
    "agents" { $targetRoots += (Join-Path $AgentsHome "skills") }
    "both" {
        $targetRoots += (Join-Path $CodexHome "skills")
        $targetRoots += (Join-Path $AgentsHome "skills")
    }
}

function Copy-SkillTree {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceSkillDir,
        [Parameter(Mandatory = $true)]
        [string]$TargetSkillsRoot
    )

    $skillName = Split-Path -Leaf $SourceSkillDir
    $targetSkillDir = Join-Path $TargetSkillsRoot $skillName

    New-Item -ItemType Directory -Force -Path $targetSkillDir | Out-Null

    Get-ChildItem -Path $SourceSkillDir -Recurse -File | ForEach-Object {
        $relativePath = $_.FullName.Substring($SourceSkillDir.Length).TrimStart('\', '/')
        $destination = Join-Path $targetSkillDir $relativePath
        $destinationDir = Split-Path -Parent $destination
        if (-not (Test-Path $destinationDir)) {
            New-Item -ItemType Directory -Force -Path $destinationDir | Out-Null
        }
        Copy-Item -Path $_.FullName -Destination $destination -Force
    }

    Write-Output ("synced {0} -> {1}" -f $skillName, $targetSkillDir)
}

foreach ($targetRoot in $targetRoots) {
    New-Item -ItemType Directory -Force -Path $targetRoot | Out-Null
    foreach ($skillName in $skillNames) {
        $sourceSkillDir = Join-Path $sourceRoot $skillName
        if (-not (Test-Path $sourceSkillDir)) {
            throw "missing source skill directory: $sourceSkillDir"
        }
        Copy-SkillTree -SourceSkillDir $sourceSkillDir -TargetSkillsRoot $targetRoot
    }
}

Write-Output "skill sync complete"
