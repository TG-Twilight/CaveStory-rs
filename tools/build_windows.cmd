@echo off
setlocal
set "VSROOT=C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools"
set "TARGET=%~1"
set "VSARCH=x64"
if "%TARGET%"=="i686-pc-windows-msvc" set "VSARCH=x86"
call "%VSROOT%\Common7\Tools\VsDevCmd.bat" -arch=%VSARCH% -host_arch=x64
if errorlevel 1 exit /b %errorlevel%
set "PATH=%USERPROFILE%\.cargo\bin;%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%PATH%"
if "%TARGET%"=="aarch64-pc-windows-msvc" (
    set "ARMTOOLS=%~dp0..\.cache\toolchains\msvc-arm64\VC\Tools\MSVC\14.29.30133"
    set "CMAKE_GENERATOR=Ninja"
)
if defined ARMTOOLS (
    if not exist "%ARMTOOLS%\bin\Hostx64\arm64\cl.exe" exit /b 2
    set "PATH=%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%"
    set "CC_aarch64_pc_windows_msvc=%ARMTOOLS%\bin\Hostx64\arm64\cl.exe"
    set "CXX_aarch64_pc_windows_msvc=%ARMTOOLS%\bin\Hostx64\arm64\cl.exe"
    set "AR_aarch64_pc_windows_msvc=%ARMTOOLS%\bin\Hostx64\arm64\lib.exe"
    set "CARGO_TARGET_AARCH64_PC_WINDOWS_MSVC_LINKER=%ARMTOOLS%\bin\Hostx64\arm64\link.exe"
    set "CMAKE_TOOLCHAIN_FILE_aarch64_pc_windows_msvc=%~dp0windows-arm64.cmake"
)
if defined ARMTOOLS (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0cargo_windows_arm64.ps1"
    exit /b
)
cargo build --release --locked --bin CaveStory-rs --target %TARGET%
exit /b %errorlevel%
