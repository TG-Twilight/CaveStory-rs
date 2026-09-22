param([ValidateSet('Debug', 'Release')][string]$Variant = 'Release', [string]$TestSuffix = '.debug', [string]$BatchDir, [ValidatePattern('^\d{8}$')][string]$BuildDate = (Get-Date -Format 'yyyyMMdd'))
$ErrorActionPreference = 'Stop'
$runs = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../CaveStory-rs-runs'))
if (-not $BatchDir) { $BatchDir = Join-Path $runs "builds/$(Get-Date -Format 'yyyyMMdd-HHmmss-fff')" }
$BatchDir = [IO.Path]::GetFullPath($BatchDir)
if (-not $BatchDir.StartsWith($runs.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'BatchDir must stay within CaveStory-rs-runs' }
$env:CAVESTORY_BUILD_VERSION = $BuildDate
$outputDir = Join-Path $BatchDir 'android'
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
$env:CAVESTORY_RUNS = $runs
$env:CARGO_TARGET_DIR = Join-Path $runs 'cache/cargo'
$env:REVIA_KS_PASS = [Environment]::GetEnvironmentVariable('REVIA_KS_PASS', 'User')
if ([string]::IsNullOrEmpty($env:REVIA_KS_PASS)) { throw 'Missing Windows user environment variable REVIA_KS_PASS' }
$env:JAVA_HOME = 'C:\Program Files\Android\Android Studio\jbr'
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:JAVA_HOME\bin;$env:PATH"
Push-Location "$PSScriptRoot/../platforms/android"
try {
    & py "$PSScriptRoot/package_windows_with_game.py" --android-assets "$BatchDir/android/bundled-assets"
    if ($LASTEXITCODE -ne 0) { throw 'Bundled game asset preparation failed' }
    $hashes = @()
    foreach ($packageKind in @('base', 'game')) {
    $assetArguments = @()
    if ($packageKind -eq 'game') { $assetArguments += "-PcaveStoryGameAssets=$BatchDir/android/bundled-assets" }
    & ./gradlew.bat "assemble$Variant" @assetArguments "-PcaveStoryBuildDate=$BuildDate" "-PcaveStoryTestSuffix=$TestSuffix" --project-cache-dir "$runs/cache/gradle-project" --console=plain
    if ($LASTEXITCODE -ne 0) { throw "Android build failed: $LASTEXITCODE" }

    $variantName = $Variant.ToLowerInvariant()
    # Without IDE-injected ABI filtering, AGP publishes splits here. Do not read
    # stale intermediates/apk metadata left by earlier single-ABI IDE builds.
    $metadataPath = "$runs/cache/android/app/build/outputs/apk/$variantName/output-metadata.json"
    $metadata = Get-Content -Raw -LiteralPath $metadataPath | ConvertFrom-Json
    if ($metadata.elements.Count -ne 2) { throw 'Expected exactly two ABI APKs' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    foreach ($targetAbi in @('arm64-v8a', 'armeabi-v7a')) {
        $element = @($metadata.elements | Where-Object {
            @($_.filters | Where-Object { $_.filterType -eq 'ABI' -and $_.value -eq $targetAbi }).Count -eq 1
        })
        if ($element.Count -ne 1) { throw "Missing or duplicate APK for $targetAbi" }
        $sourceApk = Join-Path (Split-Path $metadataPath) $element[0].outputFile
        $zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $sourceApk))
        try {
            $nativeLibs = @($zip.Entries | Where-Object { $_.FullName -match '^lib/[^/]+/[^/]+\.so$' })
            if ($nativeLibs.Count -eq 0 -or @($nativeLibs | Where-Object {
                -not $_.FullName.StartsWith("lib/$targetAbi/")
            }).Count -ne 0) { throw "Wrong native ABI contents for $targetAbi" }
            foreach ($native in $nativeLibs) {
                $stream = $native.Open()
                try {
                    $header = New-Object byte[] 20
                    $read = 0
                    while ($read -lt 20) { $count = $stream.Read($header, $read, 20 - $read); if ($count -le 0) { throw 'Truncated ELF' }; $read += $count }
                    $class = if ($targetAbi -eq 'arm64-v8a') { 2 } else { 1 }
                    $machine = if ($targetAbi -eq 'arm64-v8a') { 183 } else { 40 }
                    if ($header[0] -ne 127 -or $header[1] -ne 69 -or $header[2] -ne 76 -or $header[3] -ne 70 -or $header[4] -ne $class -or [BitConverter]::ToUInt16($header,18) -ne $machine) { throw "Wrong ELF: $($native.FullName)" }
                } finally { $stream.Dispose() }
            }
        } finally { $zip.Dispose() }
        $verification = & "$env:ANDROID_HOME/build-tools/35.0.1/apksigner.bat" verify --verbose --print-certs $sourceApk
        if ($LASTEXITCODE -ne 0 -or -not ($verification -match 'ea385afc82e19824eea8a8868ed365df03e9886bbba33e47e1e04b43ec65cb67')) {
            throw "APK signature verification failed for $targetAbi"
        }
        $abiDir = Join-Path $outputDir "$targetAbi/$packageKind"
        New-Item -ItemType Directory -Force -Path $abiDir | Out-Null
        $targetApk = Join-Path $abiDir "CaveStory-rs_android_${BuildDate}_${targetAbi}$(if ($packageKind -eq 'game') { '_game' }).apk"
        Copy-Item -LiteralPath $sourceApk -Destination $targetApk
        $verification | Set-Content -Encoding UTF8 (Join-Path $outputDir "$targetAbi-$packageKind-signature.txt")
        $hashes += "$((Get-FileHash -LiteralPath $targetApk -Algorithm SHA256).Hash.ToLowerInvariant())  $targetAbi/$packageKind/$(Split-Path $targetApk -Leaf)"
    }
    }
    $hashes | Set-Content -Encoding ASCII (Join-Path $outputDir 'SHA256SUMS')
    Write-Output "Verified four signed APKs: $((Resolve-Path $outputDir).Path)"
} finally {
    Pop-Location
    Remove-Item Env:REVIA_KS_PASS
}
exit 0
