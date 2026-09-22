# Prepare the hosted Windows runner for the same delivery scripts used locally.
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot
$runs = [IO.Path]::GetFullPath((Join-Path $repo '../CaveStory-rs-runs'))
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This setup is only for GitHub-hosted CI' }
"CAVESTORY_RUNS=$runs" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append

$vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vs) { throw 'Visual Studio C++ tools were not found' }
$version = (Get-Content -LiteralPath "$vs/VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt").Trim()
$armTools = "$vs/VC/Tools/MSVC/$version"
if (-not (Test-Path -LiteralPath "$armTools/bin/Hostx64/arm64/cl.exe")) { throw 'MSVC ARM64 cross compiler is missing' }
"VSROOT=$vs" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
# Export the target compiler only inside the ARM64 build.
"CAVESTORY_ARMTOOLS=$armTools" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append

$redistVersion = (Get-Content -LiteralPath "$vs/VC/Auxiliary/Build/Microsoft.VCRedistVersion.default.txt").Trim()
foreach ($arch in @('x86_64', 'x86_32', 'arm64')) {
    $redistArch = @{x86_64='x64'; x86_32='x86'; arm64='arm64'}[$arch]
    $crt = @(Get-ChildItem -LiteralPath "$vs/VC/Redist/MSVC/$redistVersion/$redistArch" -Directory -Filter 'Microsoft.VC*.CRT')
    if ($crt.Count -ne 1) { throw "Expected one CRT directory for $arch" }
    $destination = Join-Path $runs "cache/msvc-runtime/$arch"
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    $machine = @{x86_64=0x8664; x86_32=0x14c; arm64=0xaa64}[$arch]
    foreach ($dll in Get-ChildItem -LiteralPath $crt[0].FullName -Filter '*.dll') {
        $bytes = [IO.File]::ReadAllBytes($dll.FullName)
        $pe = [BitConverter]::ToInt32($bytes, 0x3c)
        if ([BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550) { throw "Invalid runtime PE: $($dll.Name)" }
        # Microsoft's ARM64 redist also carries compatibility DLLs for other
        # machines. Ship only native files; the delivery tests check them again.
        if ([BitConverter]::ToUInt16($bytes, $pe + 4) -eq $machine) {
            Copy-Item -LiteralPath $dll.FullName -Destination $destination
        }
    }
    if (-not (Test-Path -LiteralPath "$destination/vcruntime140.dll")) { throw "Missing native CRT for $arch" }
}

"JAVA_HOME=$env:JAVA_HOME_17_X64" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
if (-not (Test-Path -LiteralPath "$env:JAVA_HOME_17_X64/bin/java.exe")) { throw 'JDK 17 is missing' }
$env:JAVA_HOME = $env:JAVA_HOME_17_X64
$sdkmanager = "$env:ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager.bat"
& $sdkmanager --install "--package_file=$repo/platforms/android/app/packages.txt" 'cmake;3.22.1'
if ($LASTEXITCODE -ne 0) { throw 'Android SDK installation failed' }
& rustup toolchain install 1.98.1 --profile minimal
if ($LASTEXITCODE -ne 0) { throw 'Rust installation failed' }
& rustup default 1.98.1
if ($LASTEXITCODE -ne 0) { throw 'Rust selection failed' }
& rustup target add x86_64-pc-windows-msvc i686-pc-windows-msvc aarch64-pc-windows-msvc aarch64-linux-android armv7-linux-androideabi
if ($LASTEXITCODE -ne 0) { throw 'Rust target installation failed' }
& py -3 -m pip install -r "$repo/tools/requirements-font.txt"
if ($LASTEXITCODE -ne 0) { throw 'Font tooling installation failed' }
