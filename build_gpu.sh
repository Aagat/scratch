#!/bin/bash

# GPU-Accelerated Vanity ID Generator Build Script
# For Apple Silicon Macs with Metal support

set -e

echo "🚀 Building GPU-Accelerated Vanity ID Generator"
echo "================================================"

# Check if we're on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ Error: This GPU implementation requires macOS (Apple Silicon)"
    exit 1
fi

# Check if we're on Apple Silicon
if [[ $(uname -m) != "arm64" ]]; then
    echo "⚠️  Warning: This is optimized for Apple Silicon. You may not have GPU acceleration."
fi

echo "📦 Building project..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
else
    echo "❌ Build failed!"
    exit 1
fi

echo ""
echo "🧪 Running GPU initialization test..."
cargo test gpu::tests::test_gpu_initialization -- --nocapture

echo ""
echo "🎯 Testing GPU functionality with simple prefix..."
echo "Running: cargo run --release -- --gpu --prefix 'a' --gpu-batch-size 50000"
cargo run --release -- --gpu --prefix "a" --gpu-batch-size 50000

echo ""
echo "📊 Performance comparison test..."
echo ""
echo "CPU Performance (8 threads):"
time cargo run --release -- --prefix "ab" --cores 8

echo ""
echo "GPU Performance:"
time cargo run --release -- --gpu --prefix "ab" --gpu-batch-size 1000000

echo ""
echo "🎉 GPU build and test complete!"
echo ""
echo "Usage examples:"
echo "  ./target/release/vanity-id-rust --gpu --prefix 'hello'"
echo "  ./target/release/vanity-id-rust --gpu --prefix 'test' --gpu-batch-size 2000000"
echo ""
echo "For more information, see README.gpu.md"
