# CI runner only: this check invokes Cargo. Never execute on the user's PC.
param([string]$Root = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location -LiteralPath $Root
try {
    $json = & cargo metadata --offline --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $metadata = ($json -join "`n") | ConvertFrom-Json
    $expected = @{
        'taiyou-types' = @()
        'taiyou-ass' = @('taiyou-types')
        'taiyou-font' = @('taiyou-types')
        'taiyou-render' = @('taiyou-font', 'taiyou-types')
        'taiyou-capi' = @('taiyou-ass', 'taiyou-render')
        'taiyou-cli' = @('taiyou-ass', 'taiyou-render')
        'xtask' = @('taiyou-cli')
    }
    $packages = @($metadata.packages)
    if ($packages.Count -ne 7 -or $metadata.workspace_members.Count -ne 7) {
        throw 'Expected exactly seven packages and workspace members'
    }
    $names = @($packages.name | Sort-Object)
    if (@(Compare-Object $names @($expected.Keys | Sort-Object)).Count) {
        throw 'Workspace package names differ from architecture A3'
    }
    foreach ($package in $packages) {
        if ($package.edition -ne '2024') { throw "Wrong edition: $($package.name)" }
        if ($package.publish.Count -ne 0) { throw "Package must remain unpublished: $($package.name)" }
        if ($package.source -or $metadata.workspace_members -cnotcontains $package.id) {
            throw "Non-workspace package: $($package.name)"
        }
        $actual = @($package.dependencies | ForEach-Object {
            if ($_.source -or -not $_.path -or $_.kind -or $_.optional -or $_.target -or $_.rename) {
                throw "Unexpected external, renamed, optional or conditional dependency in $($package.name)"
            }
            $_.name
        } | Sort-Object)
        $wanted = @($expected[$package.name] | Sort-Object)
        if (($actual -join ',') -cne ($wanted -join ',')) {
            throw "Dependency graph mismatch for $($package.name): [$actual], expected [$wanted]"
        }
    }
    # Cargo validates manifest syntax; resolver is not included in metadata JSON.
    $manifest = Get-Content -Raw -LiteralPath (Join-Path $Root 'Cargo.toml')
    if ($manifest -notmatch '(?m)^resolver\s*=\s*"3"\s*$') { throw 'Expected workspace resolver 3' }
    Write-Output 'PASS: exact seven-package A3 DAG, local-only dependencies, edition 2024, resolver 3, unpublished packages.'
} finally {
    Pop-Location
}
