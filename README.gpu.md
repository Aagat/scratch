# GPU-Accelerated Vanity ID Generator

This is a GPU-accelerated version of the Chrome extension vanity ID generator, specifically optimized for Apple Silicon Macs using Metal compute shaders.

## Features

- **GPU Acceleration**: Utilizes Apple Silicon's GPU cores for massive parallel computation
- **Metal Compute Shaders**: Custom Metal shaders implement SHA-256 hashing and prefix matching on GPU
- **Fallback Support**: Automatically falls back to CPU implementation if GPU initialization fails
- **Configurable Batch Sizes**: Tune GPU workload for optimal performance

## Requirements

- **Apple Silicon Mac** (M1, M2, M3, or newer)
- **macOS** with Metal support
- **Rust** toolchain

## Usage

### Basic GPU Usage

```bash
# Use GPU acceleration with default settings
cargo run -- --gpu --prefix "hello"

# Specify custom batch size for GPU
cargo run -- --gpu --prefix "test" --gpu-batch-size 2000000
```

### Command Line Options

```
--gpu                              Use GPU acceleration (Apple Silicon only)
--gpu-batch-size <GPU_BATCH_SIZE>  GPU batch size for each compute dispatch [default: 1000000]
```

### Performance Comparison

```bash
# CPU version (8 cores)
cargo run -- --prefix "ab"

# GPU version
cargo run -- --gpu --prefix "ab"
```

## Performance

The GPU implementation can achieve significant speedups over CPU-only implementations:

- **Apple M3**: ~40,000+ keys/second (varies by prefix difficulty)
- **Batch Size**: Larger batch sizes (1M-10M) typically provide better GPU utilization
- **Memory**: GPU implementation uses shared memory buffers for efficient data transfer

## Architecture

### GPU Compute Pipeline

1. **Metal Shader Compilation**: The Metal compute shader is compiled at runtime
2. **Buffer Management**: Input/output data is managed through Metal buffers
3. **Parallel Execution**: Each GPU thread processes one potential key
4. **Atomic Results**: First match wins using atomic operations
5. **Data Transfer**: Results are transferred back to CPU memory

### Key Components

- `src/vanity_id.metal`: Metal compute shader with SHA-256 implementation
- `src/gpu.rs`: Rust wrapper for Metal GPU operations
- `src/main.rs`: Integration with existing CLI application

### Metal Shader Features

- **Custom SHA-256**: Full SHA-256 implementation in Metal Shading Language
- **Prefix Matching**: Optimized character mapping and comparison
- **Key Generation**: Deterministic key generation from counter values
- **Atomic Synchronization**: Thread-safe result reporting

## Troubleshooting

### GPU Initialization Fails

If you see "Failed to initialize GPU", the application will automatically fall back to CPU mode. This can happen if:

- Running on non-Apple Silicon hardware
- Metal drivers are not available
- Insufficient GPU memory

### Performance Tuning

- **Batch Size**: Try different `--gpu-batch-size` values (100K to 10M)
- **Prefix Length**: Shorter prefixes are found faster
- **System Load**: Close other GPU-intensive applications

### Memory Usage

The GPU implementation uses minimal memory:

- Input buffers: ~few KB for prefix and parameters
- Output buffer: ~44 bytes for results
- Shader compilation: One-time cost at startup

## Building

```bash
# Build with GPU support (macOS only)
cargo build --release

# Run tests
cargo test

# Check GPU functionality
cargo test gpu::tests::test_gpu_initialization -- --nocapture
```

## Benchmarking

Compare CPU vs GPU performance:

```bash
# Benchmark CPU (8 threads)
time cargo run --release -- --prefix "test"

# Benchmark GPU
time cargo run --release -- --gpu --prefix "test"
```

## Technical Details

### Metal Compute Shader

The Metal shader implements:

- Complete SHA-256 algorithm with proper padding
- Character mapping for Chrome extension IDs
- Efficient prefix matching with early termination
- 32-bit atomic operations for result synchronization

### Buffer Layout

```
Results Buffer (11 x u32):
[0] found_flag (0 or 1)
[1] counter_low (lower 32 bits)
[2] counter_high (upper 32 bits)
[3-10] key_data (8 x u32 chunks)
```

### Thread Organization

- **Thread Groups**: Up to 1024 threads per group (hardware dependent)
- **Grid Size**: Calculated based on batch size and max threads per group
- **Work Distribution**: Each thread processes one counter value

## Limitations

- **Apple Silicon Only**: Requires Metal-compatible Apple Silicon Mac
- **Single GPU**: Uses default system GPU device
- **Memory Constraints**: Very large batch sizes may hit GPU memory limits
- **Compilation Time**: Metal shader compilation adds startup overhead

## Future Improvements

- Support for multiple GPU devices
- Adaptive batch size tuning
- GPU memory usage optimization
- Cross-platform compute shader support (OpenCL/CUDA)
