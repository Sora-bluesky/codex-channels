[CmdletBinding()]
param()

function Get-RemottyPlanningRootMarkerPath {
    $explicitMarkerPath = [Environment]::GetEnvironmentVariable('REMOTTY_PLANNING_ROOT_MARKER')
    if (-not [string]::IsNullOrWhiteSpace($explicitMarkerPath)) {
        return $explicitMarkerPath
    }

    $localAppData = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { [Environment]::GetFolderPath('LocalApplicationData') }
    return Join-Path $localAppData 'remotty\planning-root.txt'
}

function Get-RemottyPlanningRootFromMarker {
    $markerPath = Get-RemottyPlanningRootMarkerPath
    if (-not (Test-Path -LiteralPath $markerPath)) {
        return $null
    }

    try {
        $markerValue = (Get-Content -LiteralPath $markerPath -Raw -ErrorAction Stop).Trim()
        if (-not [string]::IsNullOrWhiteSpace($markerValue)) {
            return $markerValue
        }
    } catch {
        return $null
    }

    return $null
}

function Find-RemottyPlanningRoot {
    param([Parameter(Mandatory = $true)][string]$UserProfile)

    try {
        $backlogFiles = Get-ChildItem -LiteralPath $UserProfile -Filter 'backlog.yaml' -File -Recurse -Depth 8 -ErrorAction SilentlyContinue
        $candidates = New-Object System.Collections.Generic.List[string]
        foreach ($file in $backlogFiles) {
            $directory = $file.DirectoryName
            if ([string]::IsNullOrWhiteSpace($directory)) {
                continue
            }

            if ($directory -notmatch '[\\/]remotty[\\/]planning$') {
                continue
            }

            if (-not (Test-Path -LiteralPath (Join-Path $directory 'ROADMAP.md'))) {
                continue
            }

            if (-not (Test-Path -LiteralPath (Join-Path $directory 'roadmap-title-ja.psd1'))) {
                continue
            }

            $candidates.Add($directory) | Out-Null
        }

        if ($candidates.Count -eq 0) {
            return $null
        }

        $preferred = @(
            $candidates | Sort-Object `
                @{ Expression = { if ($_ -match '[\\/]iCloudDrive[\\/]iCloud~md~obsidian[\\/]MainVault[\\/]Projects[\\/]remotty[\\/]planning$') { 0 } else { 1 } } }, `
                @{ Expression = { $_.Length } }, `
                @{ Expression = { $_ } }
        )

        if ($preferred.Count -gt 0) {
            return [string]$preferred[0]
        }
    } catch {
        return $null
    }

    return $null
}

function Get-RemottyDefaultPlanningRoot {
    $cachedPlanningRoot = $null
    $cachedVariable = Get-Variable -Scope Script -Name RemottyDefaultPlanningRoot -ErrorAction SilentlyContinue
    if ($cachedVariable) {
        $cachedPlanningRoot = [string]$cachedVariable.Value
    }

    if (-not [string]::IsNullOrWhiteSpace($cachedPlanningRoot)) {
        return $cachedPlanningRoot
    }

    $userProfile = if ($env:USERPROFILE) { $env:USERPROFILE } else { [Environment]::GetFolderPath('UserProfile') }
    $markerRoot = Get-RemottyPlanningRootFromMarker
    if (-not [string]::IsNullOrWhiteSpace($markerRoot)) {
        $script:RemottyDefaultPlanningRoot = $markerRoot
        return $script:RemottyDefaultPlanningRoot
    }

    $discoveredRoot = Find-RemottyPlanningRoot -UserProfile $userProfile
    if (-not [string]::IsNullOrWhiteSpace($discoveredRoot)) {
        $script:RemottyDefaultPlanningRoot = $discoveredRoot
        return $script:RemottyDefaultPlanningRoot
    }

    $script:RemottyDefaultPlanningRoot = Join-Path $userProfile '.remotty\planning'
    return $script:RemottyDefaultPlanningRoot
}

function Get-RemottyPlanningRoot {
    if (-not [string]::IsNullOrWhiteSpace($env:REMOTTY_PLANNING_ROOT)) {
        return $env:REMOTTY_PLANNING_ROOT
    }

    return Get-RemottyDefaultPlanningRoot
}

function Resolve-RemottyPlanningFilePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepoRoot,

        [Parameter(Mandatory = $true)]
        [string]$LocalRelativePath,

        [Parameter(Mandatory = $true)]
        [string]$EnvironmentVariable,

        [Parameter(Mandatory = $true)]
        [string]$DefaultFileName
    )

    $explicitPath = [Environment]::GetEnvironmentVariable($EnvironmentVariable)
    if (-not [string]::IsNullOrWhiteSpace($explicitPath)) {
        return $explicitPath
    }

    $externalPath = Join-Path (Get-RemottyPlanningRoot) $DefaultFileName
    $localPath = Join-Path $RepoRoot $LocalRelativePath

    if ((Test-Path -LiteralPath $externalPath) -or -not (Test-Path -LiteralPath $localPath)) {
        return $externalPath
    }

    return $localPath
}

function Resolve-RemottyExternalPlanningFilePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$EnvironmentVariable,

        [Parameter(Mandatory = $true)]
        [string]$DefaultFileName
    )

    $explicitPath = [Environment]::GetEnvironmentVariable($EnvironmentVariable)
    if (-not [string]::IsNullOrWhiteSpace($explicitPath)) {
        return $explicitPath
    }

    return Join-Path (Get-RemottyPlanningRoot) $DefaultFileName
}
