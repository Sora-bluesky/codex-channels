[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [string]$ReviewPath = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-common.ps1')

function Get-ReviewPath {
    param(
        [string]$ExplicitPath,
        [string]$Tag
    )

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        return $ExplicitPath
    }

    if (-not [string]::IsNullOrWhiteSpace($env:REMOTTY_RELEASE_DOC_REVIEW_PATH)) {
        return $env:REMOTTY_RELEASE_DOC_REVIEW_PATH
    }

    return Join-Path (Join-Path $env:LOCALAPPDATA 'remotty\release-doc-reviews') "$Tag.psd1"
}

$tag = Get-ReleaseTag -Version $Version
$resolvedPath = Get-ReviewPath -ExplicitPath $ReviewPath -Tag $tag

if (-not (Test-Path -LiteralPath $resolvedPath)) {
    throw "Release documentation review is required before $tag. Create a review record at $resolvedPath."
}

$review = Import-PowerShellDataFile -LiteralPath $resolvedPath
$requiredDocs = @('README.md', 'README.ja.md')

if ([string]($review.Version) -ne $tag) {
    throw "Release documentation review version mismatch. Expected $tag, got '$($review.Version)'."
}

if ([string]($review.EnglishReviewStatus) -ne 'approved') {
    throw "English public documentation review must be approved before $tag."
}

if ([string]($review.JapaneseReviewStatus) -ne 'approved') {
    throw "Japanese public documentation review must be approved before $tag."
}

if ([string]($review.JapaneseReviewerModel) -ne 'claude-opus-4-7') {
    throw "Japanese public documentation must be reviewed with claude-opus-4-7 before $tag."
}

$reviewedDocs = @($review.ReviewedDocs | ForEach-Object { [string]$_ })
foreach ($doc in $requiredDocs) {
    if ($reviewedDocs -notcontains $doc) {
        throw "Release documentation review for $tag must include $doc."
    }
}

if ([string]::IsNullOrWhiteSpace([string]$review.Notes)) {
    throw "Release documentation review for $tag must include Notes."
}

Write-Output "release documentation review gate passed for $tag"
