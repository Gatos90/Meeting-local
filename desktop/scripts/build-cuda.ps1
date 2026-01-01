# Build Windows installer with CUDA (NVIDIA GPU) support
$ErrorActionPreference = "Stop"

Write-Host "=== Building Meeting-Local with CUDA Support ===" -ForegroundColor Cyan
Write-Host ""

# === PREREQUISITE CHECKS ===
Write-Host "[Prerequisites] Checking required tools..." -ForegroundColor Yellow

$missing = @()

# Check Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $missing += "Rust (https://rustup.rs)"
}

# Check Node/pnpm
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    $missing += "pnpm (npm install -g pnpm)"
}

# Check CUDA
if (-not $env:CUDA_PATH) {
    $missing += "CUDA Toolkit - CUDA_PATH not set (https://developer.nvidia.com/cuda-downloads)"
}

# Check CMake
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    $missing += "CMake (https://cmake.org/download)"
}

# Check Visual Studio
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) {
    $missing += "Visual Studio with C++ workload (https://visualstudio.microsoft.com)"
} else {
    $vsPath = & $vswhere -latest -property installationPath
    if (-not $vsPath) {
        $missing += "Visual Studio C++ Build Tools (install 'Desktop development with C++' workload)"
    }
}

if ($missing.Count -gt 0) {
    Write-Host "`nERROR: Missing prerequisites:" -ForegroundColor Red
    foreach ($item in $missing) {
        Write-Host "  - $item" -ForegroundColor Red
    }
    Write-Host "`nInstall missing tools and try again." -ForegroundColor Yellow
    exit 1
}

Write-Host "  Rust: OK" -ForegroundColor Green
Write-Host "  pnpm: OK" -ForegroundColor Green
Write-Host "  CMake: OK" -ForegroundColor Green
Write-Host "  CUDA: $env:CUDA_PATH" -ForegroundColor Green
Write-Host "  Visual Studio: $vsPath" -ForegroundColor Green

# Navigate to desktop directory
Set-Location $PSScriptRoot\..
$desktopDir = (Get-Location).Path

# Set up VS dev environment for CUDA/nvcc
Write-Host "`nSetting up Visual Studio environment..." -ForegroundColor Gray
$vsDevShellModule = Join-Path $vsPath "Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
Import-Module $vsDevShellModule
Enter-VsDevShell -VsInstallPath $vsPath -DevCmdArguments "-arch=x64" -SkipAutomaticLocation
Set-Location $desktopDir

# Set NVCC_CCBIN to cl.exe directly so nvcc doesn't try to run vcvars64.bat
$clPath = (Get-Command cl.exe).Source
$env:NVCC_CCBIN = Split-Path $clPath
Write-Host "  NVCC_CCBIN: $env:NVCC_CCBIN" -ForegroundColor Gray

# Clean old sidecar binaries to ensure fresh build
Write-Host "`n[1/5] Cleaning old sidecar binaries..." -ForegroundColor Yellow
Remove-Item -Path "src-tauri\binaries\llm-sidecar-*" -Force -ErrorAction SilentlyContinue

# === BUILD STEPS ===
Write-Host "`n[2/5] Building frontend..." -ForegroundColor Yellow
pnpm build

Write-Host "`n[3/5] Building LLM sidecar with CUDA..." -ForegroundColor Yellow

# Save PATH and set minimal PATH to avoid CMD line length limit when vcvars64.bat is called
$originalPath = $env:PATH
$msvcBin = Get-ChildItem "$vsPath\VC\Tools\MSVC\*\bin\HostX64\x64" -Directory | Select-Object -First 1
$pnpmDir = Split-Path (Get-Command pnpm).Source
$nodeDir = Split-Path (Get-Command node).Source
$cmakeDir = Split-Path (Get-Command cmake).Source
$minimalPath = @(
    "$env:CUDA_PATH\bin",
    $msvcBin.FullName,
    "$env:SystemRoot\System32",
    "$env:SystemRoot",
    "$env:USERPROFILE\.cargo\bin",
    "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin",
    $pnpmDir,
    $nodeDir,
    $cmakeDir
) -join ";"
$env:PATH = $minimalPath
Write-Host "  Using minimal PATH for CUDA build" -ForegroundColor Gray

Push-Location "$desktopDir\src-tauri"
cargo build --release -p llm-sidecar --features cuda
$sidecarResult = $LASTEXITCODE
Pop-Location

# Restore PATH
$env:PATH = $originalPath

if ($sidecarResult -ne 0) {
    Write-Host "ERROR: Sidecar build failed" -ForegroundColor Red
    exit 1
}

Write-Host "`n[4/5] Building Tauri app with CUDA..." -ForegroundColor Yellow

# Use minimal PATH again for Tauri build
$env:PATH = $minimalPath

# Write temp config to skip beforeBuildCommand (already built frontend)
$tempConfig = Join-Path $env:TEMP "tauri-build-config.json"
'{"build":{"beforeBuildCommand":""}}' | Out-File -FilePath $tempConfig -Encoding ascii
pnpm tauri build --config $tempConfig -- --features cuda
$tauriResult = $LASTEXITCODE
Remove-Item $tempConfig -Force -ErrorAction SilentlyContinue

# Restore PATH
$env:PATH = $originalPath

if ($tauriResult -ne 0) {
    Write-Host "ERROR: Tauri build failed" -ForegroundColor Red
    exit 1
}

Write-Host "`n[5/5] Renaming installer..." -ForegroundColor Yellow
$bundleDir = "src-tauri\target\release\bundle\nsis"
# Only target the fresh build artifact (meeting-local_*.exe)
# This prevents picking up already renamed files (e.g. meeting-local-Vulkan_*)
$originalExe = Get-ChildItem "$bundleDir\meeting-local_*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1

if ($originalExe) {
    # Check if this is already a renamed file to be safe (though pattern matching should prevent it)
    if ($originalExe.Name -notmatch "CUDA|Vulkan") {
        $newName = $originalExe.Name -replace "meeting-local", "meeting-local-CUDA"
        $newPath = Join-Path $bundleDir $newName
        Move-Item $originalExe.FullName $newPath -Force
        Write-Host "`n=== Build Complete ===" -ForegroundColor Green
        Write-Host "Output: $newPath" -ForegroundColor Cyan
    } else {
         Write-Host "WARNING: Found $originalExe but it appears to be already renamed." -ForegroundColor Yellow
    }
} else {
    Write-Host "WARNING: Could not find installer to rename (looking for meeting-local_*.exe)" -ForegroundColor Yellow
}
