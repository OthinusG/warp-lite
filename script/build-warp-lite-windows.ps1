#!/usr/bin/env pwsh
#
# Builds Windows x64 release artifacts (installer + portable zip) for Warp Lite.
#
# Prerequisites:
#   - Rust toolchain target: x86_64-pc-windows-msvc
#   - protoc installed and on PATH
#   - Inno Setup 6 (ISCC.exe on PATH)
#   - cargo-about installed and on PATH
#
# Usage:
#   pwsh script/build-warp-lite-windows.ps1 [-ReleaseTag "v0.5.7-lite"]
#
# Outputs:
#   ./WarpLiteSetup-x64.exe
#   ./WarpLite-windows-x64.zip

Param(
    [Alias('release-tag')]
    [String]$ReleaseTag = "0.1.0"
)

$ErrorActionPreference = 'Stop'

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Get-Item "$ScriptDir\..").FullName
Set-Location $RepoRoot

$Arch = "x64"
$PlatformTarget = "x86_64-pc-windows-msvc"
$CargoProfile = "rlto"
$TargetOutputDir = "$RepoRoot\target\$PlatformTarget\$CargoProfile"
$BundledResourcesDir = "$TargetOutputDir\resources"
$WindowsInstallerDir = "$RepoRoot\script\windows"

# Set environment variables for build scripts
$env:GIT_RELEASE_TAG = $ReleaseTag
$env:CARGO_BIN_NAME = "oss"
$env:WARP_APP_NAME = "WarpLite"
$env:CARGO_FULL_PROFILE = $CargoProfile

# 1. Compile warp-oss binary
Write-Host "==> [1/4] Building warp-oss binary ($PlatformTarget, profile: $CargoProfile)..."
cargo build -p warp --profile $CargoProfile --bin warp-oss --features "release_bundle,gui,nld_improvements" --target $PlatformTarget
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

$WarpOssExe = "$TargetOutputDir\warp-oss.exe"
if (-not (Test-Path $WarpOssExe)) {
    throw "Build failed: $WarpOssExe not found"
}

# 2. Prepare bundled resources
Write-Host "==> [2/4] Preparing bundled resources..."
& "$WindowsInstallerDir\prepare_bundled_resources.ps1" -DestinationDir "$BundledResourcesDir" -Channel "oss"
if ($LASTEXITCODE -ne 0) {
    throw "prepare_bundled_resources.ps1 failed with exit code $LASTEXITCODE"
}

# 3. Build Inno Setup installer
Write-Host "==> [3/4] Compiling Windows installer with Inno Setup..."
$ISCC_ARGS = @(
    "$WindowsInstallerDir\windows-installer.iss",
    "/DReleaseChannel=oss",
    "/DMyAppExeName=warp-oss.exe",
    "/DTargetProfileDir=$TargetOutputDir",
    "/DMyAppName=WarpLite",
    "/DMyAppVersion=$ReleaseTag",
    "/DArch=$Arch",
    "/DOutputName=WarpLiteSetup-x64"
)
& ISCC @ISCC_ARGS
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
}

$InstallerPath = "$WindowsInstallerDir\Output\WarpLiteSetup-x64.exe"
if (-not (Test-Path $InstallerPath)) {
    throw "Installer was not created at $InstallerPath"
}
Copy-Item $InstallerPath "$RepoRoot\WarpLiteSetup-x64.exe" -Force

# 4. Package portable zip
Write-Host "==> [4/4] Creating portable zip archive..."
$PortableDir = "$RepoRoot\target\WarpLite-portable-x64"
if (Test-Path $PortableDir) { Remove-Item $PortableDir -Recurse -Force }
New-Item -ItemType Directory -Path $PortableDir -Force | Out-Null

Copy-Item "$TargetOutputDir\warp-oss.exe" "$PortableDir\WarpLite.exe" -Force
Copy-Item "$TargetOutputDir\warp-oss.exe" "$PortableDir\warp-oss.exe" -Force
Copy-Item "$RepoRoot\app\assets\windows\x64\conpty.dll" "$PortableDir\conpty.dll" -Force
Copy-Item "$RepoRoot\app\assets\windows\x64\dxcompiler.dll" "$PortableDir\dxcompiler.dll" -Force
Copy-Item "$RepoRoot\app\assets\windows\x64\dxil.dll" "$PortableDir\dxil.dll" -Force
Copy-Item "$RepoRoot\app\assets\windows\x64\msvcp140.dll" "$PortableDir\msvcp140.dll" -Force
Copy-Item "$RepoRoot\app\assets\windows\x64\vcruntime140.dll" "$PortableDir\vcruntime140.dll" -Force
Copy-Item "$RepoRoot\app\assets\windows\x64\vcruntime140_1.dll" "$PortableDir\vcruntime140_1.dll" -Force
Copy-Item "$RepoRoot\app\assets\bundled\bootstrap\pwsh.ps1" "$PortableDir\pwsh.ps1" -Force
Copy-Item "$RepoRoot\app\channels\oss\icon\no-padding\icon.ico" "$PortableDir\icon.ico" -Force

$OpenConsoleDir = "$PortableDir\x64"
New-Item -ItemType Directory -Path $OpenConsoleDir -Force | Out-Null
Copy-Item "$RepoRoot\app\assets\windows\x64\OpenConsole.exe" "$OpenConsoleDir\OpenConsole.exe" -Force

Copy-Item "$BundledResourcesDir" "$PortableDir\resources" -Recurse -Force

$ZipPath = "$RepoRoot\WarpLite-windows-x64.zip"
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
Compress-Archive -Path "$PortableDir\*" -DestinationPath $ZipPath -Force

Write-Host "==> Windows x64 build successfully completed!"
Write-Host "    Installer: $RepoRoot\WarpLiteSetup-x64.exe"
Write-Host "    Portable:  $ZipPath"
