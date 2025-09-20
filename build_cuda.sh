#!/bin/bash

# Build script for CUDA version of vanity ID generator

set -e

echo "Building CUDA version of vanity ID generator..."

# Check if CUDA is available
if ! command -v nvcc &> /dev/null; then
    echo "Error: nvcc (CUDA compiler) not found in PATH"
    echo "Please install CUDA toolkit or add it to your PATH"
    echo "Common CUDA installation paths:"
    echo "  - /usr/local/cuda/bin"
    echo "  - /opt/cuda/bin"
    echo "  - C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v*/bin"
    exit 1
fi

echo "Found CUDA compiler: $(nvcc --version | head -n1)"

# Check for NVIDIA GPU
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected:"
    nvidia-smi --query-gpu=name,compute_cap --format=csv,noheader,nounits
else
    echo "Warning: nvidia-smi not found. Cannot verify NVIDIA GPU presence."
fi

# Set CUDA environment variables if not already set
if [ -z "$CUDA_PATH" ]; then
    # Try to find CUDA installation
    for cuda_path in /usr/local/cuda /opt/cuda; do
        if [ -d "$cuda_path" ]; then
            export CUDA_PATH="$cuda_path"
            echo "Setting CUDA_PATH to: $CUDA_PATH"
            break
        fi
    done
    
    if [ -z "$CUDA_PATH" ]; then
        echo "Warning: CUDA_PATH not set and could not auto-detect CUDA installation"
        echo "You may need to set CUDA_PATH manually"
    fi
fi

# Build with CUDA support
echo "Building with CUDA support..."
cargo build --release --features cuda

echo "Build completed successfully!"
echo ""
echo "Usage examples:"
echo "  # Use CUDA GPU only"
echo "  ./target/release/vanity-id-rust --prefix myapp --cuda"
echo ""
echo "  # Use CUDA GPU + CPU hybrid mode"
echo "  ./target/release/vanity-id-rust --prefix myapp --cuda-hybrid"
echo ""
echo "  # Adjust CUDA batch size for performance tuning"
echo "  ./target/release/vanity-id-rust --prefix myapp --cuda --cuda-batch-size 2000000"
