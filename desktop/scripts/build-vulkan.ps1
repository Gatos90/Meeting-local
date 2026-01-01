# Build Windows installer with Vulkan (AMD/Intel GPU) support
# Note: mistral.rs doesn't support Vulkan, so LLM runs on CPU
# whisper-rs uses Vulkan for transcription acceleration
$ErrorActionPreference = "Stop"

Write-Host "=== Building Meeting-Local with Vulkan Support ===" -ForegroundColor Cyan
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

# Check Vulkan SDK
if (-not $env:VULKAN_SDK) {
    $missing += "Vulkan SDK - VULKAN_SDK not set (https://vulkan.lunarg.com/sdk/home)"
}

# Check Visual Studio (needed for Rust compilation)
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
Write-Host "  Vulkan SDK: $env:VULKAN_SDK" -ForegroundColor Green
Write-Host "  Visual Studio: $vsPath" -ForegroundColor Green

# Navigate to desktop directory
Set-Location $PSScriptRoot\..
$desktopDir = (Get-Location).Path

# Clean old sidecar binaries to ensure fresh build
Write-Host "`n[1/5] Cleaning old sidecar binaries..." -ForegroundColor Yellow
Remove-Item -Path "src-tauri\binaries\llm-sidecar-*" -Force -ErrorAction SilentlyContinue

# === BUILD STEPS ===
Write-Host "`n[2/5] Building frontend..." -ForegroundColor Yellow
pnpm build

Write-Host "`n[3/5] Building LLM sidecar (CPU mode - no CUDA)..." -ForegroundColor Yellow

Push-Location "$desktopDir\src-tauri"
cargo build --release -p llm-sidecar --no-default-features
$sidecarResult = $LASTEXITCODE
Pop-Location

if ($sidecarResult -ne 0) {
    Write-Host "ERROR: Sidecar build failed" -ForegroundColor Red
    exit 1
}

Write-Host "`n[4/5] Building Tauri app with Vulkan..." -ForegroundColor Yellow

# Write temp config to skip beforeBuildCommand (already built frontend)
$tempConfig = Join-Path $env:TEMP "tauri-build-config.json"
'{"build":{"beforeBuildCommand":""}}' | Out-File -FilePath $tempConfig -Encoding ascii
pnpm tauri build --config $tempConfig -- --features vulkan
$tauriResult = $LASTEXITCODE
Remove-Item $tempConfig -Force -ErrorAction SilentlyContinue

if ($tauriResult -ne 0) {
    Write-Host "ERROR: Tauri build failed" -ForegroundColor Red
    exit 1
}

Write-Host "`n[5/5] Renaming installer..." -ForegroundColor Yellow
$bundleDir = "src-tauri\target\release\bundle\nsis"
# Only target the fresh build artifact (meeting-local_*.exe)
# This prevents picking up already renamed files (e.g. meeting-local-CUDA_*)
$originalExe = Get-ChildItem "$bundleDir\meeting-local_*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1

if ($originalExe) {
    # Check if this is already a renamed file to be safe
    if ($originalExe.Name -notmatch "CUDA|Vulkan") {
        $newName = $originalExe.Name -replace "meeting-local", "meeting-local-Vulkan"
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
