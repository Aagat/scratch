# Windows Build Instructions for CUDA Support

This guide explains how to build the vanity ID generator on Windows with CUDA GPU acceleration.

## Prerequisites

### 1. NVIDIA GPU with CUDA Support

- NVIDIA GPU with compute capability 5.0 or higher
- Supported GPUs: GTX 900 series and newer, RTX series, Tesla, Quadro
- Check compatibility: https://developer.nvidia.com/cuda-gpus

### 2. NVIDIA CUDA Toolkit

Download and install the CUDA Toolkit from NVIDIA:

- **Download**: https://developer.nvidia.com/cuda-downloads
- **Recommended versions**: CUDA 11.6, 11.7, 11.8, or 12.0+
- **Installation**: Follow the installer wizard, ensure "Development" components are selected

### 3. Microsoft Visual Studio Build Tools

CUDA requires MSVC compiler:

- **Option A**: Install Visual Studio 2019/2022 with C++ development tools
- **Option B**: Install "Build Tools for Visual Studio" (lighter option)
- **Download**: https://visualstudio.microsoft.com/downloads/

### 4. Rust Programming Language

- **Download**: https://rustup.rs/
- **Installation**: Run the installer and follow prompts
- **Verify**: Open Command Prompt and run `cargo --version`

## Build Process

### Method 1: Automated Build Script

1. **Open Command Prompt as Administrator**
2. **Navigate to project directory**:
   ```cmd
   cd path\to\vanity-id-rust
   ```
3. **Run the build script**:
   ```cmd
   build_windows_cuda.bat
   ```

### Method 2: Manual Build

1. **Verify CUDA installation**:

   ```cmd
   nvcc --version
   ```

2. **Set CUDA environment variables** (if not automatically set):

   ```cmd
   set CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.0
   ```

3. **Build with CUDA support**:
   ```cmd
   cargo build --release --features cuda
   ```

## Usage

After successful build, the executable will be located at:

```
target\release\vanity-id-rust.exe
```

### Basic Usage Examples

```cmd
# GPU acceleration (CUDA)
vanity-id-rust.exe --prefix myapp --gpu

# Hybrid mode (GPU + CPU simultaneously)
vanity-id-rust.exe --prefix myapp --hybrid

# CPU only
vanity-id-rust.exe --prefix myapp

# Performance tuning
vanity-id-rust.exe --prefix myapp --gpu --gpu-batch-size 2000000
```

### Performance Testing

Test your GPU performance:

```cmd
# Quick test with simple prefix
vanity-id-rust.exe --prefix a --gpu

# Benchmark with longer prefix
vanity-id-rust.exe --prefix hello --gpu
```

## Expected Performance

Performance varies by GPU model:

| GPU Model | Approximate Speed | Speedup vs CPU |
| --------- | ----------------- | -------------- |
| RTX 4090  | ~50M keys/sec     | 100x           |
| RTX 4080  | ~40M keys/sec     | 80x            |
| RTX 3080  | ~30M keys/sec     | 75x            |
| RTX 3070  | ~25M keys/sec     | 60x            |
| GTX 1660  | ~10M keys/sec     | 33x            |

## Troubleshooting

### Common Issues

**1. "CUDA not found" error**

- Ensure CUDA Toolkit is installed
- Verify `nvcc --version` works
- Check CUDA_PATH environment variable

**2. "nvcc compilation failed"**

- Ensure Visual Studio Build Tools are installed
- Try running from "Developer Command Prompt"
- Check GPU compute capability compatibility

**3. "Failed to initialize CUDA GPU"**

- Verify NVIDIA drivers are up to date
- Ensure GPU supports CUDA
- Check if GPU is being used by other applications

**4. Build errors with MSVC**

- Install Visual Studio Build Tools
- Ensure C++ development tools are installed
- Try building from "Developer Command Prompt"

### Debug Build

For debugging, build without optimizations:

```cmd
cargo build --features cuda
```

### Verbose Build

For detailed build information:

```cmd
cargo build --release --features cuda --verbose
```

## Cross-Compilation from Linux/macOS

To build Windows binaries from other platforms:

1. **Install Windows target**:

   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```

2. **Install MinGW-w64**:

   ```bash
   # Ubuntu/Debian
   sudo apt install mingw-w64

   # macOS
   brew install mingw-w64
   ```

3. **Cross-compile** (Note: CUDA compilation still requires Windows):
   ```bash
   cargo build --target x86_64-pc-windows-gnu --release
   ```

## Distribution

The built executable (`vanity-id-rust.exe`) can be distributed to other Windows systems with:

- NVIDIA GPU with CUDA support
- NVIDIA drivers installed
- CUDA Runtime (automatically installed with drivers)

No additional CUDA Toolkit installation required on target systems.

## Performance Optimization

### GPU Batch Size Tuning

Adjust `--gpu-batch-size` based on your GPU:

- **High-end GPUs (RTX 4090, 3080)**: 2,000,000 - 5,000,000
- **Mid-range GPUs (RTX 3070, 2070)**: 1,000,000 - 2,000,000
- **Entry-level GPUs (GTX 1660)**: 500,000 - 1,000,000

### Hybrid Mode Optimization

For maximum performance, use hybrid mode which utilizes both GPU and CPU:

```cmd
vanity-id-rust.exe --prefix myapp --hybrid --gpu-batch-size 2000000
```

## Support

For Windows-specific build issues:

1. Check NVIDIA CUDA documentation
2. Verify Visual Studio Build Tools installation
3. Ensure all prerequisites are met
4. Try building with verbose output for detailed error information
