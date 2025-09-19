@echo off
REM Windows build script for CUDA-enabled vanity ID generator
REM This script should be run on a Windows system with CUDA toolkit installed

echo Building vanity ID generator for Windows with CUDA support...

REM Check if CUDA is available
nvcc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: CUDA toolkit not found. Please install NVIDIA CUDA Toolkit.
    echo Download from: https://developer.nvidia.com/cuda-downloads
    exit /b 1
)

echo CUDA toolkit found:
nvcc --version

REM Check if Rust is available
cargo --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Rust not found. Please install Rust.
    echo Download from: https://rustup.rs/
    exit /b 1
)

echo Rust found:
cargo --version

REM Set CUDA environment variables if not already set
if not defined CUDA_PATH (
    for /f "tokens=*" %%i in ('where nvcc') do (
        set "NVCC_PATH=%%i"
        goto :found_nvcc
    )
    echo ERROR: Could not determine CUDA_PATH
    exit /b 1
    :found_nvcc
    for %%i in ("%NVCC_PATH%") do set "CUDA_PATH=%%~dpi.."
    echo Set CUDA_PATH=%CUDA_PATH%
)

REM Build with CUDA feature enabled
echo Building with CUDA support...
cargo build --release --features cuda

if %errorlevel% neq 0 (
    echo ERROR: Build failed
    exit /b 1
)

echo.
echo ✅ Build successful!
echo.
echo Executable location: target\release\vanity-id-rust.exe
echo.
echo Usage examples:
echo   vanity-id-rust.exe --prefix myapp --gpu
echo   vanity-id-rust.exe --prefix myapp --hybrid
echo   vanity-id-rust.exe --prefix myapp --gpu --gpu-batch-size 2000000
echo.
echo To test GPU functionality:
echo   target\release\vanity-id-rust.exe --prefix test --gpu
