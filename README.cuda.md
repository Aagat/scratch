# CUDA GPU Acceleration for Vanity ID Generator

This document describes the CUDA GPU acceleration implementation for the vanity ID generator, which provides massive performance improvements on NVIDIA GPUs.

## Overview

The CUDA implementation provides an alternative GPU acceleration backend for NVIDIA GPUs, complementing the existing Metal implementation for Apple Silicon. It uses CUDA kernels to perform parallel SHA-256 hashing and prefix matching directly on the GPU.

## Requirements

### Hardware

- NVIDIA GPU with CUDA Compute Capability 5.0 or higher
- Recommended: Modern NVIDIA GPU (GTX 1060/RTX 2060 or newer)
- Sufficient GPU memory (typically 1GB+ for optimal performance)

### Software

- NVIDIA CUDA Toolkit 11.0 or newer
- NVIDIA GPU drivers (compatible with your CUDA version)
- Rust toolchain
- C++ compiler (for CUDA compilation)

## Installation

### 1. Install CUDA Toolkit

#### Linux (Ubuntu/Debian)

```bash
# Add NVIDIA package repository
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2004/x86_64/cuda-keyring_1.0-1_all.deb
sudo dpkg -i cuda-keyring_1.0-1_all.deb
sudo apt-get update

# Install CUDA toolkit
sudo apt-get install cuda-toolkit-12-0
```

#### Windows

1. Download CUDA Toolkit from [NVIDIA Developer](https://developer.nvidia.com/cuda-downloads)
2. Run the installer and follow the setup wizard
3. Add CUDA to your PATH (usually done automatically)

#### macOS

CUDA is not supported on macOS with Apple Silicon. Use the Metal implementation instead.

### 2. Verify CUDA Installation

```bash
nvcc --version
nvidia-smi
```

### 3. Build with CUDA Support

```bash
# Using the build script
chmod +x build_cuda.sh
./build_cuda.sh

# Or manually
export CUDA_PATH=/usr/local/cuda  # Adjust path as needed
cargo build --release --features cuda
```

## Usage

### Command Line Options

#### CUDA GPU Only

```bash
./target/release/vanity-id-rust --prefix myapp --cuda
```

#### CUDA GPU + CPU Hybrid Mode

```bash
./target/release/vanity-id-rust --prefix myapp --cuda-hybrid
```

#### Performance Tuning

```bash
# Adjust batch size for your GPU
./target/release/vanity-id-rust --prefix myapp --cuda --cuda-batch-size 2000000

# Use fewer CPU threads in hybrid mode
./target/release/vanity-id-rust --prefix myapp --cuda-hybrid --cores 4
```

### Performance Comparison

Typical performance improvements with CUDA:

| Hardware | CPU Only | CUDA GPU | Speedup |
| -------- | -------- | -------- | ------- |
| RTX 4090 | ~500K/s  | ~50M/s   | 100x    |
| RTX 3080 | ~400K/s  | ~30M/s   | 75x     |
| GTX 1660 | ~300K/s  | ~10M/s   | 33x     |

_Performance varies based on prefix length, system configuration, and thermal conditions._

## Architecture

### CUDA Kernel Design

The CUDA implementation consists of:

1. **Kernel Function** (`vanity_search`): Main GPU kernel that processes batches of counters
2. **SHA-256 Implementation**: GPU-optimized SHA-256 hashing
3. **Key Generation**: Deterministic key data generation from counter values
4. **Prefix Matching**: Efficient string matching on GPU
5. **Result Handling**: Atomic operations for thread-safe result collection

### Memory Management

- **Device Memory**: Pre-allocated buffers for results and prefix data
- **Shared Memory**: Used for SHA-256 constants and temporary data
- **Global Memory**: Efficient coalesced access patterns
- **Memory Transfer**: Minimized host-device transfers

### Thread Organization

- **Block Size**: 256 threads per block (optimized for most GPUs)
- **Grid Size**: Dynamically calculated based on batch size
- **Thread Mapping**: Each thread processes one counter value
- **Atomic Operations**: Used for result synchronization

## Performance Optimization

### Batch Size Tuning

The `--cuda-batch-size` parameter controls how many hash operations are performed per GPU kernel launch:

- **Small batches** (100K-500K): Lower latency, good for short prefixes
- **Medium batches** (1M-2M): Balanced performance, default setting
- **Large batches** (5M+): Maximum throughput, good for long prefixes

### GPU Architecture Considerations

The implementation includes optimizations for different GPU architectures:

- **Maxwell** (GTX 900 series): Basic CUDA support
- **Pascal** (GTX 1000 series): Improved memory bandwidth
- **Volta/Turing** (RTX 2000 series): Enhanced compute capabilities
- **Ampere** (RTX 3000 series): Maximum performance
- **Ada Lovelace** (RTX 4000 series): Latest optimizations

### Hybrid Mode Strategy

When using `--cuda-hybrid`:

- GPU handles 75% of the search space
- CPU threads handle remaining 25%
- Automatic load balancing
- First result wins (GPU or CPU)

## Troubleshooting

### Common Issues

#### CUDA Not Found

```
Error: nvcc (CUDA compiler) not found in PATH
```

**Solution**: Install CUDA Toolkit and add to PATH

#### GPU Memory Issues

```
CUDA error: out of memory
```

**Solution**: Reduce batch size with `--cuda-batch-size 500000`

#### Compilation Errors

```
nvcc compilation failed
```

**Solution**: Check CUDA/compiler compatibility, update drivers

#### No GPU Detected

```
No CUDA devices found
```

**Solution**: Verify GPU drivers, check `nvidia-smi` output

### Performance Issues

#### Low GPU Utilization

- Increase batch size
- Check thermal throttling
- Verify power settings

#### Slower Than Expected

- Update GPU drivers
- Check system thermal conditions
- Try different batch sizes
- Ensure adequate power supply

## Development

### Code Structure

```
src/
├── vanity_id.cu      # CUDA kernel implementation
├── cuda_gpu.rs      # Rust wrapper for CUDA
├── main.rs          # CLI integration
└── build.rs         # Build script for CUDA compilation
```

### Building from Source

```bash
# Clone repository
git clone <repository-url>
cd vanity-id-generator

# Install dependencies
cargo build

# Build with CUDA
./build_cuda.sh
```

### Testing

```bash
# Run CUDA tests
cargo test --features cuda

# Benchmark performance
cargo run --release --features cuda -- --prefix test --cuda
```

## Limitations

- Requires NVIDIA GPU with CUDA support
- Windows/Linux only (no macOS support)
- Additional build complexity
- GPU memory constraints for very large batches

## Future Improvements

- Multi-GPU support
- OpenCL backend for AMD GPUs
- Dynamic batch size adjustment
- Memory pool optimization
- Kernel fusion optimizations

## Contributing

When contributing to the CUDA implementation:

1. Test on multiple GPU architectures
2. Benchmark performance changes
3. Maintain compatibility with existing features
4. Update documentation for new features
5. Follow CUDA best practices

## License

Same as the main project license.
