param([string]$Root = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
$rootPath = [IO.Path]::GetFullPath($Root)
$failures = [Collections.Generic.List[string]]::new()
$requiredDocs = @('AGENTS.md','README.md','docs/ARCHITECTURE.md','docs/GUIDELINES.md','docs/GOALS.md','docs/ROADMAP.md','docs/CONTRACTS.md','docs/TESTING.md','docs/COMPATIBILITY.md','docs/CACHING.md','docs/DEVELOPMENT.md','docs/SOURCES.md')
foreach ($relative in $requiredDocs) {
    if (-not (Test-Path -LiteralPath (Join-Path $rootPath $relative))) { $failures.Add("Missing $relative") }
}
$markdown = @(Get-ChildItem -LiteralPath $rootPath -Recurse -File -Filter '*.md' | Where-Object { $_.FullName -notmatch '[\\/](target|\.git|\.local|artifacts)[\\/]' })
$contents = @{}
$anchors = @{}
foreach ($file in $markdown) {
    $body = [IO.File]::ReadAllText($file.FullName)
    $contents[$file.FullName] = $body
    $withoutCode = [regex]::Replace($body, '(?ms)^```.*?^```\s*$', '')
    $headingIds = [Collections.Generic.HashSet[string]]::new()
    $duplicates = @{}
    foreach ($m in [regex]::Matches($withoutCode, '(?m)^#{1,6}\s+(.+?)\s*$')) {
        $slug = $m.Groups[1].Value.ToLowerInvariant()
        $slug = [regex]::Replace($slug, '[^\p{L}\p{Nd}_\-\s]', '')
        $slug = [regex]::Replace($slug, '\s', '-')
        if ($duplicates.ContainsKey($slug)) { $duplicates[$slug]++; $slug = "$slug-$($duplicates[$slug])" }
        else { $duplicates[$slug] = 0 }
        [void]$headingIds.Add($slug)
    }
    $anchors[$file.FullName] = $headingIds
}
$linkCount = 0
foreach ($file in $markdown) {
    foreach ($m in [regex]::Matches($contents[$file.FullName], '\[[^\]\r\n]+\]\(([^)\r\n]+)\)')) {
        $target = $m.Groups[1].Value.Trim('<','>')
        if ($target -match '^[a-zA-Z][a-zA-Z0-9+.-]*:') { continue }
        $parts = $target.Split('#', 2)
        $targetFile = if ($parts[0]) { [IO.Path]::GetFullPath((Join-Path $file.DirectoryName $parts[0])) } else { $file.FullName }
        if (-not $targetFile.StartsWith($rootPath + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            $failures.Add("Link escapes blueprint root: $($file.Name) -> $target"); continue
        }
        if (-not (Test-Path -LiteralPath $targetFile -PathType Leaf)) { $failures.Add("Broken link: $($file.Name) -> $target"); continue }
        if ($parts.Length -eq 2 -and $parts[1] -and $anchors.ContainsKey($targetFile) -and -not $anchors[$targetFile].Contains($parts[1])) {
            $failures.Add("Broken anchor: $($file.Name) -> $target")
        }
        $linkCount++
    }
}
$sections = @('Objective','Prerequisites','Files and modules','Technical contract','Expected behavior','Implementation guidance','Acceptance criteria','Verification','Forbidden changes','Completion report')
$taskFiles = @($markdown | Where-Object { $_.Directory.Name -eq 'tasks' -and $_.BaseName -match '^[TR]\d{3}$' })
$graph = @{}
foreach ($file in $taskFiles) {
    $body = $contents[$file.FullName]
    if ($body -notmatch ('(?m)^# ' + $file.BaseName + '\b')) { $failures.Add("Task title mismatch: $($file.Name)") }
    foreach ($section in $sections) {
        if ($body -notmatch ('(?m)^## ' + [regex]::Escape($section) + '\s*$')) { $failures.Add("$($file.Name) missing section $section") }
    }
    $prereq = [regex]::Match($body, '(?ms)^## Prerequisites\s*\n(.*?)(?=^## |\z)').Groups[1].Value
    $graph[$file.BaseName] = @([regex]::Matches($prereq, '\[([TR]\d{3})\]') | ForEach-Object { $_.Groups[1].Value })
}
$visited = @{}
function Visit-Task([string]$Id) {
    if ($visited[$Id] -eq 1) { $failures.Add("Dependency cycle at $Id"); return }
    if ($visited[$Id] -eq 2) { return }
    $visited[$Id] = 1
    foreach ($dep in $graph[$Id]) {
        if (-not $graph.ContainsKey($dep)) { $failures.Add("$Id has missing prerequisite $dep") }
        else { Visit-Task $dep }
    }
    $visited[$Id] = 2
}
foreach ($id in @($graph.Keys)) { Visit-Task $id }
$roadmap = [IO.File]::ReadAllText((Join-Path $rootPath 'docs/ROADMAP.md'))
$index = [IO.File]::ReadAllText((Join-Path $rootPath 'docs/tasks/README.md'))
foreach ($id in $graph.Keys) {
    if ($roadmap -notmatch "\b$id\b" -or $index -notmatch "\b$id\b") { $failures.Add("$id absent from roadmap/index") }
}
$lock = Get-Content -Raw -LiteralPath (Join-Path $rootPath 'docs/references.lock.json') | ConvertFrom-Json
if ($lock.schema -ne 1 -or $lock.references.Count -ne 4) { $failures.Add('Unexpected reference lock schema/count') }
$refIds = @{}
foreach ($reference in $lock.references) {
    if ($reference.commit -notmatch '^[0-9a-f]{40}$') { $failures.Add("Invalid revision pin: $($reference.id)") }
    if ($refIds.ContainsKey($reference.id)) { $failures.Add("Duplicate reference: $($reference.id)") }
    $refIds[$reference.id] = $true
}
$xy = @($lock.references | Where-Object id -eq 'xy-vsfilter')
if ($xy.Count -ne 1 -or $xy[0].repository -ne 'https://github.com/pinterf/xy-VSFilter') { $failures.Add('Main xy reference must be pinterf/xy-VSFilter') }
if ($failures.Count) {
    foreach ($failure in $failures) { Write-Output "FAIL: $failure" }
    exit 1
}
Write-Output "PASS: $($markdown.Count) Markdown files, $linkCount local links, $($taskFiles.Count) complete task schemas, acyclic prerequisites, four revision pins and pinterf xy authority."
Write-Output 'Structural blueprint validation only; no renderer, reference images or performance results tested.'
