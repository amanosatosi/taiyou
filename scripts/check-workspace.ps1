# CI runner only: this check invokes Cargo. Never execute on the user's PC.
param([string]$Root = (Split-Path -Parent $PSScriptRoot), [switch]$SelfTest)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# Exact T008 intake only; normal workspace edges below remain unchanged.
$external = @{
    'freetype-sys' = @('=0.23.0', $false, '')
    'libz-sys' = @('=1.1.30', $true, 'static')
    'sha2' = @('=0.11.0', $true, '')
    'icu_casemap' = @('=2.0.1', $false, 'compiled_data')
    'harfbuzz-sys' = @('=0.8.0', $false, '')
    'unicode-bidi' = @('=0.3.18', $true, '')
    'unicode-script' = @('=0.5.8', $true, '')
}
function Assert-ExternalDependency($packageName, $dependency) {
    if ($packageName -cne 'taiyou-font' -or -not $external.ContainsKey($dependency.name)) { throw 'Unapproved external dependency' }
    $spec = $external[$dependency.name]
    $externalPath = if ($dependency -is [System.Collections.IDictionary]) { $dependency['path'] } elseif ($dependency.PSObject.Properties['path']) { $dependency.path } else { $null }
    if ($dependency.source -cne 'registry+https://github.com/rust-lang/crates.io-index' -or
        $dependency.req -cne $spec[0] -or $dependency.uses_default_features -ne $spec[1] -or
        (($dependency.features | Sort-Object) -join ',') -cne $spec[2] -or
        -not $dependency.optional -or $dependency.kind -or $dependency.target -or $dependency.rename -or $externalPath) {
        throw 'External dependency version/source/features/ownership mismatch'
    }
}
if ($SelfTest) {
    $sample = @{name='freetype-sys'; source='registry+https://github.com/rust-lang/crates.io-index'; req='=0.23.0'; uses_default_features=$false; features=@(); optional=$true; kind=$null; target=$null; rename=$null; path=$null}
    Assert-ExternalDependency 'taiyou-font' $sample
    foreach ($mutation in @(@{name='unknown'}, @{req='0.23'}, @{optional=$false}, @{features=@('bundled')}, @{target='cfg(windows)'}, @{rename='alias'}, @{kind='build'}, @{source='git+https://example.invalid/native'}, @{uses_default_features=$true})) {
        $changed = $sample.Clone()
        foreach ($key in $mutation.Keys) { $changed[$key] = $mutation[$key] }
        $rejected = $false
        try { Assert-ExternalDependency 'taiyou-font' $changed } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dependency tamper self-test failed' }
    }
    $rejected = $false
    try { Assert-ExternalDependency 'taiyou-render' $sample } catch { $rejected = $true }
    if (-not $rejected) { throw 'Dependency owner self-test failed' }
    Write-Output 'PASS: exact dependency admission and ten tamper scenarios.'
}
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
            if ($_.source) {
                Assert-ExternalDependency $package.name $_
            } else {
            if ($_.source -or -not $_.path -or $_.kind -or $_.optional -or $_.target -or $_.rename) {
                throw "Unexpected external, renamed, optional or conditional dependency in $($package.name)"
            }
            $_.name
            }
        } | Sort-Object)
        $wanted = @($expected[$package.name] | Sort-Object)
        if (($actual -join ',') -cne ($wanted -join ',')) {
            throw "Dependency graph mismatch for $($package.name): [$actual], expected [$wanted]"
        }
    }
    # Cargo validates manifest syntax; resolver is not included in metadata JSON.
    $manifest = Get-Content -Raw -LiteralPath (Join-Path $Root 'Cargo.toml')
    if ($manifest -notmatch '(?m)^resolver\s*=\s*"3"\s*$') { throw 'Expected workspace resolver 3' }
    Write-Output 'PASS: exact seven-package A3 DAG, exact optional T008 intake, edition 2024, resolver 3, unpublished packages.'
} finally {
    Pop-Location
}
