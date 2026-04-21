[CmdletBinding()]
param(
    [string]$PlanningRoot = '',
    [string]$MarkerPath = ''
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'planning-paths.ps1')

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

function Test-RemottyPathInsideRoot {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootPath,
        [Parameter(Mandatory = $true)]
        [string]$CandidatePath
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($RootPath)
    $resolvedCandidate = [System.IO.Path]::GetFullPath($CandidatePath)
    $rootWithSeparator = if ($resolvedRoot.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $resolvedRoot
    } else {
        $resolvedRoot + [System.IO.Path]::DirectorySeparatorChar
    }

    return $resolvedCandidate.Equals($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
        $resolvedCandidate.StartsWith($rootWithSeparator, [System.StringComparison]::OrdinalIgnoreCase)
}

function New-RemottyBootstrapBacklog {
    return @"
# === v0.1.0: Bootstrap ===
- id: TASK-001
    title: Describe the first milestone
    status: backlog
    priority: P0
    target_version: v0.1.0
    repo: remotty
"@
}

function New-RemottyBootstrapTitleMap {
    return @"
@{
    VersionTitles = @{
        "v0.1.0" = "初期計画"
    }
    TaskTitles = @{
        "TASK-001" = "初期マイルストーンを定義する"
    }
}
"@
}

if ([string]::IsNullOrWhiteSpace($PlanningRoot)) {
    $PlanningRoot = Get-RemottyPlanningRoot
}

if ([string]::IsNullOrWhiteSpace($MarkerPath)) {
    $MarkerPath = Get-RemottyPlanningRootMarkerPath
}

$resolvedPlanningRoot = if ([System.IO.Path]::IsPathRooted($PlanningRoot)) { $PlanningRoot } else { Join-Path (Get-Location).Path $PlanningRoot }
$resolvedMarkerPath = if ([System.IO.Path]::IsPathRooted($MarkerPath)) { $MarkerPath } else { Join-Path (Get-Location).Path $MarkerPath }

if (Test-RemottyPathInsideRoot -RootPath $repoRoot -CandidatePath $resolvedPlanningRoot) {
    throw "Planning root must stay outside the repository: $resolvedPlanningRoot"
}

New-Item -ItemType Directory -Force -Path $resolvedPlanningRoot | Out-Null

$backlogTarget = Join-Path $resolvedPlanningRoot 'backlog.yaml'
$titleTarget = Join-Path $resolvedPlanningRoot 'roadmap-title-ja.psd1'
$backlogAlreadyExists = Test-Path -LiteralPath $backlogTarget

if (-not $backlogAlreadyExists) {
    [System.IO.File]::WriteAllText($backlogTarget, (New-RemottyBootstrapBacklog), $utf8NoBom)
}

if (-not (Test-Path -LiteralPath $titleTarget)) {
    if ($backlogAlreadyExists) {
        [System.IO.File]::WriteAllText($titleTarget, "@{`n    VersionTitles = @{} `n    TaskTitles = @{} `n}`n", $utf8NoBom)
    } else {
        [System.IO.File]::WriteAllText($titleTarget, (New-RemottyBootstrapTitleMap), $utf8NoBom)
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
