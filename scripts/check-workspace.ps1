# CI runner only: this check invokes Cargo. Never execute on the user's PC.
param([string]$Root = (Split-Path -Parent $PSScriptRoot), [switch]$SelfTest)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# Exact native/performance intake; normal workspace edges remain unchanged.
$external = @{
    'freetype-sys' = @('=0.23.0', $false, '')
    'libz-sys' = @('=1.1.30', $true, 'static')
    'sha2' = @('=0.11.0', $true, '')
    'icu_casemap' = @('=2.0.1', $false, 'compiled_data')
    'harfbuzz-sys' = @('=0.8.0', $false, '')
    'unicode-bidi' = @('=0.3.18', $true, '')
    'unicode-script' = @('=0.5.8', $true, '')
    'encoding_rs' = @('=0.8.35', $false, '')
    'png' = @('=0.18.1', $false, '')
    'rayon' = @('=1.11.0', $true, '')
}
function Assert-ExternalDependency($packageName, $dependency) {
    $owners = switch -CaseSensitive ($dependency.name) {
        'png' { 'taiyou-cli' }
        'rayon' { 'taiyou-render' }
        'sha2' { 'taiyou-font'; 'taiyou-cli' }
        default { 'taiyou-font' }
    }
    if ($owners -cnotcontains $packageName -or -not $external.ContainsKey($dependency.name)) { throw 'Unapproved external dependency' }
    $spec = $external[$dependency.name]
    $optional = $dependency.name -cne 'rayon'
    $externalPath = if ($dependency -is [System.Collections.IDictionary]) { $dependency['path'] } elseif ($dependency.PSObject.Properties['path']) { $dependency.path } else { $null }
    if ($dependency.source -cne 'registry+https://github.com/rust-lang/crates.io-index' -or
        $dependency.req -cne $spec[0] -or $dependency.uses_default_features -ne $spec[1] -or
        (($dependency.features | Sort-Object) -join ',') -cne $spec[2] -or
        $dependency.optional -ne $optional -or $dependency.kind -or $dependency.target -or $dependency.rename -or $externalPath) {
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
    foreach ($name in @('harfbuzz-sys','unicode-bidi','unicode-script','encoding_rs')) {
        $admitted = $sample.Clone()
        $admitted.name = $name
        $admitted.req = $external[$name][0]
        $admitted.uses_default_features = $external[$name][1]
        Assert-ExternalDependency 'taiyou-font' $admitted
        foreach ($mutation in @(@{req='*'}, @{uses_default_features=(-not $admitted.uses_default_features)}, @{features=@('freetype')})) {
            $changed = $admitted.Clone()
            foreach ($key in $mutation.Keys) { $changed[$key] = $mutation[$key] }
            $rejected = $false
            try { Assert-ExternalDependency 'taiyou-font' $changed } catch { $rejected = $true }
            if (-not $rejected) { throw 'T009 exact dependency tamper test failed' }
        }
    }
    Write-Output 'PASS: exact shaping/DBCS dependencies with twelve additional tamper scenarios.'
    $png = $sample.Clone()
    $png.name = 'png'; $png.req = '=0.18.1'; $png.uses_default_features = $false
    Assert-ExternalDependency 'taiyou-cli' $png
    foreach ($mutation in @(@{req='*'}, @{uses_default_features=$true}, @{features=@('unstable')}, @{optional=$false}, @{source='git+https://example.invalid/png'})) {
        $changed = $png.Clone()
        foreach ($key in $mutation.Keys) { $changed[$key] = $mutation[$key] }
        $rejected = $false
        try { Assert-ExternalDependency 'taiyou-cli' $changed } catch { $rejected = $true }
        if (-not $rejected) { throw 'T014 PNG dependency tamper test failed' }
    }
    $rejected = $false
    try { Assert-ExternalDependency 'taiyou-font' $png } catch { $rejected = $true }
    if (-not $rejected) { throw 'T014 PNG owner tamper test failed' }
    $rayon = $sample.Clone()
    $rayon.name = 'rayon'; $rayon.req = '=1.11.0'; $rayon.uses_default_features = $true; $rayon.optional = $false
    Assert-ExternalDependency 'taiyou-render' $rayon
    foreach ($mutation in @(@{req='1.11'}, @{uses_default_features=$false}, @{features=@('web_spin_lock')}, @{optional=$true}, @{source='git+https://example.invalid/rayon'}, @{target='cfg(windows)'}, @{rename='pool'}, @{kind='build'}, @{path='/tmp/pool'})) {
        $changed = $rayon.Clone()
        foreach ($key in $mutation.Keys) { $changed[$key] = $mutation[$key] }
        $rejected = $false
        try { Assert-ExternalDependency 'taiyou-render' $changed } catch { $rejected = $true }
        if (-not $rejected) { throw 'Performance pool dependency tamper test failed' }
    }
    foreach ($owner in @('taiyou-font','taiyou-cli','taiyou-types')) {
        $rejected = $false
        try { Assert-ExternalDependency $owner $rayon } catch { $rejected = $true }
        if (-not $rejected) { throw 'Performance pool owner tamper test failed' }
    }
    $audit = $sample.Clone()
    $audit.name = 'sha2'; $audit.req = '=0.11.0'; $audit.uses_default_features = $true
    Assert-ExternalDependency 'taiyou-font' $audit
    Assert-ExternalDependency 'taiyou-cli' $audit
    foreach ($mutation in @(@{req='*'}, @{features=@('asm')}, @{optional=$false}, @{source='git+https://example.invalid/hash'})) {
        $changed = $audit.Clone()
        foreach ($key in $mutation.Keys) { $changed[$key] = $mutation[$key] }
        $rejected = $false
        try { Assert-ExternalDependency 'taiyou-cli' $changed } catch { $rejected = $true }
        if (-not $rejected) { throw 'Performance digest dependency tamper test failed' }
    }
    $rejected = $false
    try { Assert-ExternalDependency 'taiyou-render' $audit } catch { $rejected = $true }
    if (-not $rejected) { throw 'Performance digest owner tamper test failed' }
    Write-Output 'PASS: exact renderer pool and CLI digest edges with seventeen additional tamper scenarios.'
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
    Write-Output 'PASS: exact seven-package A3 DAG, exact native/performance intake, edition 2024, resolver 3, unpublished packages.'
} finally {
    Pop-Location
}
