$ErrorActionPreference = 'Stop'

$releaseTag = '3.2.0.810'
$assetId = 147296997
$assetUrl = 'https://github.com/pinterf/xy-VSFilter/releases/download/3.2.0.810/xyVSFilterSubFilter-pfmod-3.2.0.810.7z'
$archivePath = Join-Path $env:RUNNER_TEMP 'xy-vsfilter-release.7z'
$releaseRoot = Join-Path $env:RUNNER_TEMP 'xy-vsfilter-release'
$outputRoot = Join-Path $env:RUNNER_TEMP 'xy-oracle'
$identityPath = Join-Path $PSScriptRoot 'release.json'

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
    Invoke-WebRequest -Uri $assetUrl -OutFile $archivePath
}

$archiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
$identityEstablished = $false
if (Test-Path -LiteralPath $identityPath -PathType Leaf) {
    $identity = Get-Content -LiteralPath $identityPath -Raw | ConvertFrom-Json
    if ($identity.archive_sha256) {
        if ($identity.archive_sha256.ToLowerInvariant() -ne $archiveHash) {
            throw "xy-VSFilter archive SHA-256 mismatch: expected $($identity.archive_sha256), got $archiveHash"
        }
        $identityEstablished = $true
    }
}

if (Test-Path -LiteralPath $releaseRoot) {
    throw "Extraction directory already exists: $releaseRoot"
}
New-Item -ItemType Directory -Path $releaseRoot | Out-Null
$sevenZip = Join-Path $env:ProgramFiles '7-Zip\7z.exe'
if (-not (Test-Path -LiteralPath $sevenZip -PathType Leaf)) {
    throw "Runner 7-Zip executable not found: $sevenZip"
}
& $sevenZip x $archivePath "-o$releaseRoot" -y
if ($LASTEXITCODE -ne 0) {
    throw "7-Zip extraction failed with exit code $LASTEXITCODE"
}

$files = @(
    Get-ChildItem -LiteralPath $releaseRoot -File -Recurse | Sort-Object FullName | ForEach-Object {
        $relativePath = [System.IO.Path]::GetRelativePath($releaseRoot, $_.FullName).Replace('\', '/')
        $record = [ordered]@{
            path = $relativePath
            size = $_.Length
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        if ($_.Extension -ieq '.dll') {
            $version = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($_.FullName)
            $record.file_version = $version.FileVersion
            $record.product_version = $version.ProductVersion
        }
        [pscustomobject]$record
    }
)

$environment = Get-CimInstance -ClassName Win32_OperatingSystem
$runnerVersion = if ($env:ImageVersion) { $env:ImageVersion } else { 'unknown' }
$acquisition = [ordered]@{
    schema = 1
    release_tag = $releaseTag
    release_asset_id = $assetId
    url = $assetUrl
    archive_sha256 = $archiveHash
    identity_established = $identityEstablished
    architecture = $env:PROCESSOR_ARCHITECTURE
    windows = [ordered]@{
        caption = $environment.Caption
        version = $environment.Version
        build_number = $environment.BuildNumber
    }
    runner = [ordered]@{
        image = 'windows-2022'
        image_version = $runnerVersion
    }
}

$acquisition | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $outputRoot 'release-acquisition.json') -Encoding utf8
$files | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $outputRoot 'release-files.json') -Encoding utf8
Write-Output "xy-VSFilter $releaseTag asset $assetId SHA-256 $archiveHash; identity established: $identityEstablished"
