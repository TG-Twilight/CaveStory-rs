$ErrorActionPreference = 'Stop'
$batchTime = Get-Date
$buildDate = $batchTime.ToString('yyyyMMdd')
$env:CAVESTORY_BUILD_VERSION = $buildDate
$batch = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../../CaveStory-rs-runs/builds/$($batchTime.ToString('yyyyMMdd-HHmmss-fff'))"))
New-Item -ItemType Directory -Path $batch | Out-Null
@{ build_date=$buildDate; version=$buildDate; configuration='Release' } | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $batch 'build-info.json')
& powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/build_android.ps1" -BatchDir $batch -BuildDate $buildDate
if ($LASTEXITCODE -ne 0) { throw 'Android build failed' }
& powershell -NoProfile -ExecutionPolicy Bypass -File "$PSScriptRoot/build_windows.ps1" -BatchDir $batch -BuildDate $buildDate
if ($LASTEXITCODE -ne 0) { throw 'Windows build failed' }
& py "$PSScriptRoot/verify_local_delivery.py" $batch
if ($LASTEXITCODE -ne 0) { throw 'Final delivery verification failed' }
