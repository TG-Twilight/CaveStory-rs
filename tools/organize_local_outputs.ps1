$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot))
$runs = [IO.Path]::GetFullPath((Join-Path $repo '../CaveStory-rs-runs'))
$record = Join-Path $runs 'maintenance/20260914'
if (Test-Path -LiteralPath (Join-Path $record 'deleted-builds.json')) { throw 'This historical cleanup already has an audit record; do not overwrite it' }
New-Item -ItemType Directory -Force -Path $record | Out-Null
function CheckedPath([string]$Path, [string]$Root) {
    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not $resolved.StartsWith($Root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "Unsafe target: $resolved" }
    if (Test-Path -LiteralPath $resolved) {
        if ((Get-Item -LiteralPath $resolved -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse point: $resolved" }
    }
    return $resolved
}
$obsolete = @('20260913-184108-162-android-debug','20260913-200635-739-android-debug',
    '20260913-201758-040-windows-release','20260913-202014-657-windows-release',
    '20260913-controller','20260913-feedback','20260913-localized','20260913-mi9-compat','20260913-mi9-compat-v2')
$deletions = @()
foreach ($name in $obsolete) {
    $dir = CheckedPath (Join-Path $repo "artifacts/builds/$name") $repo
    if (-not (Test-Path -LiteralPath $dir)) { continue }
    $files = @(Get-ChildItem -LiteralPath $dir -File -Recurse -Force)
    if (@($files | Where-Object { $_.Name -match '^(Profile.*|settings\.json|Config\.dat)$' }).Count) { throw "Protected state in $dir" }
    foreach ($file in $files) {
        $deletions += [pscustomobject]@{ Path=$file.FullName; Bytes=$file.Length; SHA256=(Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant() }
    }
}
$deletions | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $record 'deleted-builds.json')
foreach ($name in $obsolete) {
    $dir = CheckedPath (Join-Path $repo "artifacts/builds/$name") $repo
    if (Test-Path -LiteralPath $dir) { Remove-Item -LiteralPath $dir -Recurse -Force }
}
$moves = @(
    @{ From='artifacts'; To='history/artifacts' },
    @{ From='target'; To='cache/cargo' }
)
$migration = @()
foreach ($move in $moves) {
    $source = CheckedPath (Join-Path $repo $move.From) $repo
    $dest = CheckedPath (Join-Path $runs $move.To) $runs
    if (-not (Test-Path -LiteralPath $source)) { continue }
    if (Test-Path -LiteralPath $dest) { throw "Destination already exists: $dest" }
    if ($move.From -eq 'artifacts') {
        foreach ($file in Get-ChildItem -LiteralPath $source -File -Recurse -Force) {
            $migration += [pscustomobject]@{ From=$file.FullName; To=$dest+$file.FullName.Substring($source.Length); SHA256=(Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant() }
        }
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null
    Move-Item -LiteralPath $source -Destination $dest
}
foreach ($entry in $migration) {
    if ((Get-FileHash -LiteralPath $entry.To).Hash.ToLowerInvariant() -ne $entry.SHA256) { throw "Migration hash mismatch: $($entry.To)" }
}
$migration | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $record 'migrated-files.json')
$saves = @(Get-ChildItem -LiteralPath $runs -File -Recurse -Force | Where-Object { $_.Name -match '^(Profile.*|settings\.json|Config\.dat)$' } | ForEach-Object {
    [pscustomobject]@{ Path=$_.FullName; SHA256=(Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant() }
})
$saves | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $record 'protected-state.json')
Write-Output "Removed $($deletions.Count) superseded files ($([math]::Round(($deletions | Measure-Object Bytes -Sum).Sum / 1MB, 1)) MiB); migrated and verified $($migration.Count) files; protected $($saves.Count) state files."
