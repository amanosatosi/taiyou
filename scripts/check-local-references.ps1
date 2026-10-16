param([string]$ReferenceRoot)
$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'docs/local-sources.json') | ConvertFrom-Json
if ($manifest.schema -ne 1) { throw 'Unsupported local source manifest schema' }
if (-not $ReferenceRoot) { $ReferenceRoot = Join-Path $repositoryRoot $manifest.root_relative_to_repository }
$referencePath = [IO.Path]::GetFullPath($ReferenceRoot)
$failures = [Collections.Generic.List[string]]::new()
$checked = 0
foreach ($source in $manifest.sources) {
    $sourcePath = [IO.Path]::GetFullPath((Join-Path $referencePath $source.directory))
    if (-not $sourcePath.StartsWith($referencePath.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        $failures.Add("Source escapes reference root: $($source.id)"); continue
    }
    foreach ($entry in $source.files) {
        $filePath = [IO.Path]::GetFullPath((Join-Path $sourcePath $entry.path))
        if (-not $filePath.StartsWith($sourcePath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            $failures.Add("File escapes source directory: $($source.id)/$($entry.path)"); continue
        }
        if (-not (Test-Path -LiteralPath $filePath -PathType Leaf)) {
            $failures.Add("Missing: $filePath"); continue
        }
        if ($entry.sha256 -notmatch '^[0-9a-f]{64}$' -or (Get-FileHash -LiteralPath $filePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) {
            $failures.Add("Fingerprint mismatch: $filePath"); continue
        }
        $checked++
    }
}
if ($failures.Count) {
    $failures | ForEach-Object { Write-Output "FAIL: $_" }
    exit 1
}
Write-Output "PASS: $($manifest.sources.Count) local source folders, $checked entry-point fingerprints."
Write-Output "Not supplied: $($manifest.missing_sources -join ', '). No network access attempted."
Write-Output 'These fingerprints do not certify complete trees, upstream commit equivalence or reference rendering results.'
