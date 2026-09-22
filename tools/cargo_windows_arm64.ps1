# Invoked within the VS host environment by build_windows.cmd. Cargo applies
# these flags only to the explicit target, leaving host build scripts on x64.
$cargo = Get-Command cargo -CommandType Application -ErrorAction Stop
$sdkLib = Join-Path $env:WindowsSdkDir "Lib/$env:WindowsSDKVersion"
$env:CARGO_ENCODED_RUSTFLAGS = @(
    "-Lnative=$env:ARMTOOLS/lib/arm64",
    "-Lnative=$sdkLib/ucrt/arm64",
    "-Lnative=$sdkLib/um/arm64"
) -join [char]31
& $cargo.Source build --release --locked --bin CaveStory-rs --target aarch64-pc-windows-msvc
exit $LASTEXITCODE
