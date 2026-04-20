[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$targets = @(
    'README.md',
    'README.ja.md'
)

$targets += @(Get-ChildItem docs -Recurse -File -Include *.md -ErrorAction SilentlyContinue | ForEach-Object {
    Resolve-Path -Relative $_.FullName
})
$targets += @(Get-ChildItem tasks -Recurse -File -Include *.md -ErrorAction SilentlyContinue | ForEach-Object {
    Resolve-Path -Relative $_.FullName
})

$bannedTerms = @(
    'winsmux',
    'remodex'
)

$failures = New-Object System.Collections.Generic.List[string]

foreach ($path in ($targets | Sort-Object -Unique)) {
    if (-not (Test-Path $path)) {
        continue
    }

    $content = Get-Content $path -Raw
    foreach ($term in $bannedTerms) {
        if ($content -match [regex]::Escape($term)) {
            $failures.Add("$path contains banned term '$term'") | Out-Null
        }
    }
}

if ($failures.Count -gt 0) {
    Write-Error ("documentation terminology audit failed:`n- " + ($failures -join "`n- "))
    exit 1
}

Write-Output 'documentation terminology audit passed'
