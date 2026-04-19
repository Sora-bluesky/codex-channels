[CmdletBinding()]
param(
    [string]$BacklogPath = '',
    [string]$RoadmapTitleJaPath = ''
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'planning-paths.ps1')

function Resolve-WorkspacePath {
    param([Parameter(Mandatory = $true)][string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path (Get-Location).Path $Path
}

function Get-TaskBlocks {
    param([Parameter(Mandatory = $true)][string]$Content)

    $normalized = $Content -replace "`r`n", "`n"
    $lines = $normalized -split "`n"
    $blocks = New-Object System.Collections.Generic.List[object]
    $current = $null

    foreach ($line in $lines) {
        if ($line -match '^[ \t]*-[ \t]+id:[ \t]*(?<id>\S+)[ \t]*$') {
            if ($null -ne $current -and $current.Count -gt 0) {
                $blocks.Add([pscustomobject]@{ Lines = @($current.ToArray()) })
            }

            $current = New-Object System.Collections.Generic.List[string]
            $current.Add($line)
            continue
        }

        if ($null -ne $current) {
            $current.Add($line)
        }
    }

    if ($null -ne $current -and $current.Count -gt 0) {
        $blocks.Add([pscustomobject]@{ Lines = @($current.ToArray()) })
    }

    return $blocks
}

function Get-TaskValues {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyString()]
        [AllowEmptyCollection()]
        [string[]]$Lines
    )

    if ($Lines[0] -notmatch '^[ \t]*-[ \t]+id:[ \t]*(?<id>\S+)[ \t]*$') {
        return $null
    }

    $values = @{
        id = $Matches['id']
    }

    for ($index = 1; $index -lt $Lines.Count; $index++) {
        $line = $Lines[$index]
        if ($line -match '^[ \t]{4}(?<key>[a-z_]+):[ \t]*(?<value>.*)$') {
            $values[$Matches['key']] = $Matches['value'].Trim()
        }
    }

    return $values
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$localBacklogPath = Join-Path $repoRoot 'tasks/backlog.example.yaml'
$localTitlePath = Join-Path $repoRoot 'tasks/roadmap-title-ja.example.psd1'
$planningRoot = Get-CodexChannelsPlanningRoot
$externalBacklogPath = Join-Path $planningRoot 'backlog.yaml'
$externalTitlePath = Join-Path $planningRoot 'roadmap-title-ja.psd1'

if ([string]::IsNullOrWhiteSpace($BacklogPath) -and [string]::IsNullOrWhiteSpace($RoadmapTitleJaPath)) {
    if ((Test-Path -LiteralPath $externalBacklogPath) -or (Test-Path -LiteralPath $externalTitlePath)) {
        $BacklogPath = $externalBacklogPath
        $RoadmapTitleJaPath = $externalTitlePath
    } else {
        $BacklogPath = $localBacklogPath
        $RoadmapTitleJaPath = $localTitlePath
    }
} elseif ([string]::IsNullOrWhiteSpace($BacklogPath)) {
    $resolvedRoadmapTitleJaPath = Resolve-WorkspacePath -Path $RoadmapTitleJaPath
    $BacklogPath = Join-Path (Split-Path -Parent $resolvedRoadmapTitleJaPath) 'backlog.yaml'
} elseif ([string]::IsNullOrWhiteSpace($RoadmapTitleJaPath)) {
    $resolvedBacklogPath = Resolve-WorkspacePath -Path $BacklogPath
    $RoadmapTitleJaPath = Join-Path (Split-Path -Parent $resolvedBacklogPath) 'roadmap-title-ja.psd1'
}

$resolvedBacklogPath = Resolve-WorkspacePath -Path $BacklogPath
$resolvedRoadmapTitleJaPath = Resolve-WorkspacePath -Path $RoadmapTitleJaPath
$failures = New-Object System.Collections.Generic.List[string]

if (-not (Test-Path -LiteralPath $resolvedBacklogPath)) {
    $failures.Add("backlog.yaml not found: $resolvedBacklogPath") | Out-Null
} else {
    $backlogContent = Get-Content -LiteralPath $resolvedBacklogPath -Raw
    $taskBlocks = @(Get-TaskBlocks -Content $backlogContent)
    if ($taskBlocks.Count -eq 0) {
        $failures.Add('backlog.yaml does not contain any task blocks') | Out-Null
    }

    $taskIds = New-Object System.Collections.Generic.HashSet[string]
    $versions = New-Object System.Collections.Generic.HashSet[string]
    $allowedStatuses = @('done', 'review', 'in-progress', 'in_progress', 'doing', 'active', 'cancelled', 'backlog')
    $allowedPriorities = @('P0', 'P1', 'P2', 'P3')
    $requiredKeys = @('title', 'status', 'priority', 'target_version', 'repo')

    foreach ($taskBlock in $taskBlocks) {
        $values = Get-TaskValues -Lines @($taskBlock.Lines)
        if ($null -eq $values) {
            $failures.Add('backlog.yaml contains a task block without a valid id line') | Out-Null
            continue
        }

        $taskId = [string]$values['id']
        if ($taskId -notmatch '^TASK-\d+$') {
            $failures.Add("task id '$taskId' must match TASK-<number>") | Out-Null
        } elseif (-not $taskIds.Add($taskId)) {
            $failures.Add("task id '$taskId' is duplicated") | Out-Null
        }

        foreach ($requiredKey in $requiredKeys) {
            if (-not $values.ContainsKey($requiredKey) -or [string]::IsNullOrWhiteSpace([string]$values[$requiredKey])) {
                $failures.Add("task '$taskId' is missing required field '$requiredKey'") | Out-Null
            }
        }

        if ($values.ContainsKey('priority') -and ($allowedPriorities -notcontains [string]$values['priority'])) {
            $failures.Add("task '$taskId' has invalid priority '$($values['priority'])'") | Out-Null
        }

        if ($values.ContainsKey('status') -and ($allowedStatuses -notcontains [string]$values['status'])) {
            $failures.Add("task '$taskId' has invalid status '$($values['status'])'") | Out-Null
        }

        if ($values.ContainsKey('target_version')) {
            $targetVersion = [string]$values['target_version']
            if ($targetVersion -notmatch '^v\d+\.\d+\.\d+$') {
                $failures.Add("task '$taskId' has invalid target_version '$targetVersion'") | Out-Null
            } else {
                [void]$versions.Add($targetVersion)
            }
        }
    }
}

if (-not (Test-Path -LiteralPath $resolvedRoadmapTitleJaPath)) {
    $failures.Add("roadmap-title-ja.psd1 not found: $resolvedRoadmapTitleJaPath") | Out-Null
} else {
    try {
        $data = Import-PowerShellDataFile -LiteralPath $resolvedRoadmapTitleJaPath
    } catch {
        $failures.Add("roadmap-title-ja.psd1 could not be parsed: $resolvedRoadmapTitleJaPath") | Out-Null
        $data = $null
    }

    if ($null -ne $data) {
        $versionTitles = if ($null -ne $data.VersionTitles) { $data.VersionTitles } else { @{} }
        $taskTitles = if ($null -ne $data.TaskTitles) { $data.TaskTitles } else { @{} }

        if ($versionTitles -isnot [System.Collections.IDictionary]) {
            $failures.Add('roadmap-title-ja.psd1 VersionTitles must be a hashtable') | Out-Null
        } else {
            foreach ($entry in $versionTitles.GetEnumerator()) {
                if ([string]::IsNullOrWhiteSpace([string]$entry.Value)) {
                    $failures.Add("VersionTitles entry '$($entry.Key)' must not be empty") | Out-Null
                }
            }
        }

        if ($taskTitles -isnot [System.Collections.IDictionary]) {
            $failures.Add('roadmap-title-ja.psd1 TaskTitles must be a hashtable') | Out-Null
        } else {
            foreach ($entry in $taskTitles.GetEnumerator()) {
                if ([string]::IsNullOrWhiteSpace([string]$entry.Value)) {
                    $failures.Add("TaskTitles entry '$($entry.Key)' must not be empty") | Out-Null
                }
            }
        }
    }
}

if ($failures.Count -gt 0) {
    Write-Error ("planning validation failed:`n- " + ($failures -join "`n- "))
    exit 1
}

Write-Output 'planning validation passed'
