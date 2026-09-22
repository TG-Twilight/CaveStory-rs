param([string]$BatchDir, [ValidatePattern('^\d{8}$')][string]$BuildDate = (Get-Date -Format 'yyyyMMdd'))
$env:CAVESTORY_BUILD_VERSION = $BuildDate
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot
$runs = [IO.Path]::GetFullPath((Join-Path $repo '../CaveStory-rs-runs'))
if (-not $BatchDir) { $BatchDir = Join-Path $runs "builds/$(Get-Date -Format 'yyyyMMdd-HHmmss-fff')" }
$BatchDir = [IO.Path]::GetFullPath($BatchDir)
if (-not $BatchDir.StartsWith($runs.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'BatchDir must stay within CaveStory-rs-runs' }
$env:CARGO_TARGET_DIR = Join-Path $runs 'cache/cargo'
Push-Location $repo
try {
    $outputDir = Join-Path $BatchDir 'windows'
    New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
    $fontDir = Join-Path $outputDir 'font-support'
    $fontArchive = Join-Path $runs 'downloads/fusion-pixel-font-12px-monospaced-bdf-v2026.09.01.zip'
    & py "$PSScriptRoot/build_chinese_font.py" --archive $fontArchive --output $fontDir
    if ($LASTEXITCODE -ne 0) { throw 'Chinese font generation failed' }
    & py "$PSScriptRoot/build_chinese_font.py" --archive $fontArchive --japanese --output "$fontDir/japanese"
    if ($LASTEXITCODE -ne 0) { throw 'Japanese font generation failed' }
    $targets = @(
        @{ Name='x86_64'; Triple='x86_64-pc-windows-msvc'; Machine=0x8664 },
        @{ Name='x86_32'; Triple='i686-pc-windows-msvc'; Machine=0x14c },
        @{ Name='arm64'; Triple='aarch64-pc-windows-msvc'; Machine=0xaa64 }
    )
    $hashes = @()
    foreach ($target in $targets) {
        & "$PSScriptRoot/build_windows.cmd" $target.Triple
        if ($LASTEXITCODE -ne 0) { throw "Windows $($target.Name) build failed: $LASTEXITCODE" }
        $sourceExe = Join-Path $env:CARGO_TARGET_DIR "$($target.Triple)/release/CaveStory-rs.exe"
        $bytes = [IO.File]::ReadAllBytes($sourceExe)
        $pe = [BitConverter]::ToInt32($bytes, 0x3c)
        if ([BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550 -or
            [BitConverter]::ToUInt16($bytes, $pe + 4) -ne $target.Machine) {
            throw "Wrong PE architecture for $($target.Name)"
        }
        $archDir = Join-Path $outputDir "$($target.Name)/engine"
        New-Item -ItemType Directory -Force -Path $archDir | Out-Null
        $folder = Join-Path $archDir "CaveStory-rs_windows_${BuildDate}_$($target.Name)"
        New-Item -ItemType Directory -Path $folder | Out-Null
        Copy-Item -LiteralPath $sourceExe -Destination (Join-Path $folder 'CaveStory-rs.exe')
        Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination $folder
        Copy-Item -LiteralPath (Join-Path $repo 'vendor/trainer/notices/TRAINER-LICENSES.txt') -Destination $folder
        # Only font and locale support assets; the original game data stays external.
        $dataDir = Join-Path $folder 'data'
        New-Item -ItemType Directory -Path $dataDir | Out-Null
        Copy-Item -LiteralPath $fontDir -Destination (Join-Path $dataDir 'fonts') -Recurse
        Copy-Item -LiteralPath (Join-Path $repo 'res/chinese/locale') -Destination $dataDir -Recurse
        $readme = @'
CaveStory-rs — based on doukutsu-rs, original game by Studio Pixel.
This package contains the engine, Chinese menu translation and OFL fonts.
Original game data is not included. Use your existing compatible game directory.
Requires the Microsoft Visual C++ runtime matching this package architecture.
Extract into a working copy of the 2004 simplified Chinese game, preserving saves.
Merge the supplied data/fonts and data/locale support files into that copy.
Keep the original Doukutsu.exe for resource extraction; run CaveStory-rs.exe.
Select Simplified Chinese in Options > Language if an explicit language is saved.
Setup instructions: https://github.com/TG-Twilight/CaveStory-rs#play
'@
        $readme | Set-Content -Encoding UTF8 (Join-Path $folder 'README.txt')
        "PE Machine: 0x$('{0:x4}' -f $target.Machine)" | Set-Content (Join-Path $folder 'architecture.txt')
        $zip = "$folder.zip"
        Compress-Archive -LiteralPath $folder -DestinationPath $zip
        & py "$PSScriptRoot/package_windows_with_game.py" --engine $sourceExe --output "$outputDir/$($target.Name)/with-game" --arch $target.Name --date $BuildDate
        if ($LASTEXITCODE -ne 0) { throw "Windows $($target.Name) complete package failed" }
        $hashes += "$((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant())  $($target.Name)/engine/$(Split-Path $zip -Leaf)"
        $hashes += "$((Get-FileHash -LiteralPath (Join-Path $folder 'CaveStory-rs.exe') -Algorithm SHA256).Hash.ToLowerInvariant())  $($target.Name)/engine/$(Split-Path $folder -Leaf)/CaveStory-rs.exe"
    }
    $hashes | Set-Content -Encoding ASCII (Join-Path $outputDir 'SHA256SUMS')
    Write-Output "Verified three Windows architectures: $outputDir"
} finally { Pop-Location }
