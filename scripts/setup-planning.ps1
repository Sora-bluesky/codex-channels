[CmdletBinding()]
param(
    [string]$PlanningRoot = '',
    [string]$MarkerPath = ''
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'planning-paths.ps1')

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

if ([string]::IsNullOrWhiteSpace($PlanningRoot)) {
    $PlanningRoot = Get-CodexChannelsPlanningRoot
}

if ([string]::IsNullOrWhiteSpace($MarkerPath)) {
    $MarkerPath = Get-CodexChannelsPlanningRootMarkerPath
}

$resolvedPlanningRoot = if ([System.IO.Path]::IsPathRooted($PlanningRoot)) { $PlanningRoot } else { Join-Path (Get-Location).Path $PlanningRoot }
$resolvedMarkerPath = if ([System.IO.Path]::IsPathRooted($MarkerPath)) { $MarkerPath } else { Join-Path (Get-Location).Path $MarkerPath }

New-Item -ItemType Directory -Force -Path $resolvedPlanningRoot | Out-Null

$backlogTarget = Join-Path $resolvedPlanningRoot 'backlog.yaml'
$titleTarget = Join-Path $resolvedPlanningRoot 'roadmap-title-ja.psd1'
$backlogAlreadyExists = Test-Path -LiteralPath $backlogTarget

if (-not $backlogAlreadyExists) {
    Copy-Item -LiteralPath (Join-Path $repoRoot 'tasks/backlog.example.yaml') -Destination $backlogTarget
}

if (-not (Test-Path -LiteralPath $titleTarget)) {
    if ($backlogAlreadyExists) {
        [System.IO.File]::WriteAllText($titleTarget, "@{`n    VersionTitles = @{} `n    TaskTitles = @{} `n}`n", $utf8NoBom)
    } else {
        Copy-Item -LiteralPath (Join-Path $repoRoot 'tasks/roadmap-title-ja.example.psd1') -Destination $titleTarget
    }
}

$validatePlanningScript = Join-Path $repoRoot 'scripts/validate-planning.ps1'
& $validatePlanningScript `
    -BacklogPath $backlogTarget `
    -RoadmapTitleJaPath $titleTarget

$syncRoadmapScript = Join-Path $repoRoot 'scripts/sync-roadmap.ps1'
& $syncRoadmapScript `
    -BacklogPath $backlogTarget `
    -RoadmapPath (Join-Path $resolvedPlanningRoot 'ROADMAP.md') `
    -RoadmapTitleJaPath $titleTarget

$markerDirectory = Split-Path -Parent $resolvedMarkerPath
if (-not [string]::IsNullOrWhiteSpace($markerDirectory)) {
    New-Item -ItemType Directory -Force -Path $markerDirectory | Out-Null
}

[System.IO.File]::WriteAllText($resolvedMarkerPath, $resolvedPlanningRoot, $utf8NoBom)

Write-Output ("Planning root: {0}" -f $resolvedPlanningRoot)
Write-Output ("Marker path: {0}" -f $resolvedMarkerPath)
