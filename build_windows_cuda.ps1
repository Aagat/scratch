# PowerShell build script for CUDA-enabled vanity ID generator
# This script should be run on a Windows system with CUDA toolkit installed

Write-Host "Building vanity ID generator for Windows with CUDA support..." -ForegroundColor Green

# Check if CUDA is available
try {
    $nvccVersion = nvcc --version 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "CUDA not found"
    }
    Write-Host "CUDA toolkit found:" -ForegroundColor Green
    Write-Host $nvccVersion
} catch {
    Write-Host "ERROR: CUDA toolkit not found. Please install NVIDIA CUDA Toolkit." -ForegroundColor Red
    Write-Host "Download from: https://developer.nvidia.com/cuda-downloads" -ForegroundColor Yellow
    exit 1
}

# Check if Rust is available
try {
    $cargoVersion = cargo --version 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "Rust not found"
    }
    Write-Host "Rust found:" -ForegroundColor Green
    Write-Host $cargoVersion
} catch {
    Write-Host "ERROR: Rust not found. Please install Rust." -ForegroundColor Red
    Write-Host "Download from: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Set CUDA environment variables if not already set
if (-not $env:CUDA_PATH) {
    try {
        $nvccPath = Get-Command nvcc -ErrorAction Stop | Select-Object -ExpandProperty Source
        $cudaPath = Split-Path (Split-Path $nvccPath -Parent) -Parent
        $env:CUDA_PATH = $cudaPath
        Write-Host "Set CUDA_PATH=$env:CUDA_PATH" -ForegroundColor Yellow
    } catch {
        Write-Host "ERROR: Could not determine CUDA_PATH" -ForegroundColor Red
        exit 1
    }
}

# Check for Visual Studio Build Tools
$vsInstallations = @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools",
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\BuildTools",
    "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community",
    "${env:ProgramFiles}\Microsoft Visual Studio\2022\Professional",
    "${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise"
)

$vsFound = $false
foreach ($vsPath in $vsInstallations) {
    if (Test-Path $vsPath) {
        Write-Host "Found Visual Studio at: $vsPath" -ForegroundColor Green
        $vsFound = $true
        break
    }
}

if (-not $vsFound) {
    Write-Host "WARNING: Visual Studio Build Tools not found in common locations." -ForegroundColor Yellow
    Write-Host "CUDA compilation may fail. Please ensure Visual Studio Build Tools are installed." -ForegroundColor Yellow
}

# Build with CUDA feature enabled
Write-Host "Building with CUDA support..." -ForegroundColor Green
Write-Host "Running: cargo build --release --features cuda" -ForegroundColor Cyan

$buildResult = cargo build --release --features cuda
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Build failed" -ForegroundColor Red
    Write-Host "Try the following troubleshooting steps:" -ForegroundColor Yellow
    Write-Host "1. Ensure Visual Studio Build Tools are installed" -ForegroundColor Yellow
    Write-Host "2. Run from 'Developer Command Prompt for VS'" -ForegroundColor Yellow
    Write-Host "3. Check CUDA and GPU compatibility" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "✅ Build successful!" -ForegroundColor Green
Write-Host ""
Write-Host "Executable location: target\release\vanity-id-rust.exe" -ForegroundColor Cyan
Write-Host ""
Write-Host "Usage examples:" -ForegroundColor Yellow
Write-Host "  .\target\release\vanity-id-rust.exe --prefix myapp --gpu" -ForegroundColor White
Write-Host "  .\target\release\vanity-id-rust.exe --prefix myapp --hybrid" -ForegroundColor White
Write-Host "  .\target\release\vanity-id-rust.exe --prefix myapp --gpu --gpu-batch-size 2000000" -ForegroundColor White
Write-Host ""
Write-Host "To test GPU functionality:" -ForegroundColor Yellow
Write-Host "  .\target\release\vanity-id-rust.exe --prefix test --gpu" -ForegroundColor White
Write-Host ""

# Optional: Run a quick test
$runTest = Read-Host "Would you like to run a quick GPU test? (y/N)"
if ($runTest -eq "y" -or $runTest -eq "Y") {
    Write-Host "Running quick GPU test..." -ForegroundColor Green
    .\target\release\vanity-id-rust.exe --prefix a --gpu
}
