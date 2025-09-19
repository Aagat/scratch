# Unified GPU Implementation - Cross-Platform Guide

This document describes the unified GPU implementation that automatically selects the appropriate GPU backend based on the platform.

## Overview

The vanity ID generator now supports GPU acceleration on all major platforms with a unified interface:

- **macOS**: Uses Metal (Apple Silicon)
- **Windows**: Uses CUDA (NVIDIA GPUs)
- **Linux**: Uses CUDA (NVIDIA GPUs)

## Unified Command Interface

### Single `--gpu` Flag

```bash
# Automatically uses Metal on macOS, CUDA on Windows/Linux
./vanity-id-rust --prefix myapp --gpu
```

### Hybrid Mode

```bash
# GPU + CPU simultaneously for maximum performance
./vanity-id-rust --prefix myapp --hybrid
```

### Performance Tuning

```bash
# Adjust GPU batch size for optimal performance
./vanity-id-rust --prefix myapp --gpu --gpu-batch-size 2000000
```

## Platform-Specific Setup

### macOS (Metal)

- **Requirements**: Apple Silicon Mac (M1, M2, M3, M4)
- **Setup**: No additional installation required
- **Build**: `cargo build --release`

### Windows (CUDA)

- **Requirements**: NVIDIA GPU + CUDA Toolkit + Visual Studio Build Tools
- **Setup**: See `README.windows.md` for detailed instructions
- **Build**: `build_windows_cuda.bat` or `build_windows_cuda.ps1`

### Linux (CUDA)

- **Requirements**: NVIDIA GPU + CUDA Toolkit + GCC
- **Setup**: Install CUDA toolkit from NVIDIA
- **Build**: `cargo build --release --features cuda`

## Performance Expectations

| Platform | GPU Type | Expected Speed | Speedup vs CPU |
| -------- | -------- | -------------- | -------------- |
| macOS    | M3 Max   | ~40M keys/sec  | 50x            |
| macOS    | M2 Pro   | ~25M keys/sec  | 35x            |
| macOS    | M1       | ~20M keys/sec  | 30x            |
| Windows  | RTX 4090 | ~50M keys/sec  | 100x           |
| Windows  | RTX 3080 | ~30M keys/sec  | 75x            |
| Windows  | GTX 1660 | ~10M keys/sec  | 33x            |
| Linux    | RTX 4090 | ~50M keys/sec  | 100x           |
| Linux    | RTX 3080 | ~30M keys/sec  | 75x            |

## Error Handling

The unified implementation provides clear error messages and **no fallback behavior**:

- GPU initialization failure results in immediate exit
- Clear error messages guide users to resolve issues
- No automatic fallback to CPU mode

### Common Error Messages

**macOS:**

```
Failed to initialize GPU: Metal device not found. This requires Apple Silicon.
```

**Windows/Linux:**

```
Failed to initialize CUDA GPU: No CUDA devices found. This requires an NVIDIA GPU with CUDA support.
```

## Build Instructions by Platform

### macOS

```bash
# Standard build (includes Metal support)
cargo build --release

# Test GPU functionality
./target/release/vanity-id-rust --prefix test --gpu
```

### Windows

```cmd
# Automated build
build_windows_cuda.bat

# Or PowerShell
.\build_windows_cuda.ps1

# Manual build
cargo build --release --features cuda
```

### Linux

```bash
# Install CUDA toolkit first, then:
cargo build --release --features cuda

# Test GPU functionality
./target/release/vanity-id-rust --prefix test --gpu
```

## Architecture Details

### Conditional Compilation

The implementation uses Rust's conditional compilation features:

```rust
#[cfg(target_os = "macos")]
// Metal implementation

#[cfg(not(target_os = "macos"))]
// CUDA implementation
```

### Unified Function Signatures

Both Metal and CUDA implementations provide identical APIs:

```rust
// Both backends implement these functions:
fn run_gpu_vanity_id_generator(prefix: &str, batch_size: u64)
fn run_hybrid_vanity_id_generator(prefix: &str, cpu_threads: usize, batch_size: u64)
```

### Build System

The build system automatically:

- Detects available GPU toolkits
- Compiles appropriate shaders/kernels
- Links required libraries
- Provides clear error messages when dependencies missing

## Troubleshooting

### macOS Issues

- Ensure you have Apple Silicon (M1/M2/M3/M4)
- Intel Macs are not supported for GPU acceleration

### Windows Issues

- Install NVIDIA CUDA Toolkit
- Install Visual Studio Build Tools
- Ensure NVIDIA drivers are up to date
- Run from Developer Command Prompt if build fails

### Linux Issues

- Install NVIDIA CUDA Toolkit
- Ensure GCC is installed
- Check CUDA_PATH environment variable
- Verify GPU compute capability compatibility

## Development Notes

### Adding New GPU Backends

To add support for additional GPU backends (e.g., OpenCL, Vulkan):

1. Create new module (e.g., `opencl_gpu.rs`)
2. Implement unified interface functions
3. Add conditional compilation directives
4. Update build system to detect new dependencies
5. Add platform-specific documentation

### Testing

The implementation includes comprehensive testing:

- Compilation tests for all platforms
- GPU initialization tests
- Performance benchmarks
- Error handling verification

## Future Enhancements

Potential improvements:

- **AMD GPU Support**: ROCm/HIP implementation for AMD GPUs
- **Intel GPU Support**: OpenCL implementation for Intel Arc GPUs
- **Vulkan Compute**: Cross-platform Vulkan compute shaders
- **Auto-tuning**: Automatic batch size optimization
- **Multi-GPU**: Support for multiple GPUs simultaneously

## Contributing

When contributing to the GPU implementation:

1. Maintain unified interface compatibility
2. Add appropriate conditional compilation
3. Update build system for new dependencies
4. Add platform-specific documentation
5. Include performance benchmarks
