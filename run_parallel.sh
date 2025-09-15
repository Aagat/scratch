#!/bin/bash

# Script to run multiple instances of vanity-id-rust in single-threaded mode using GNU parallel
# Usage: ./run_parallel.sh [prefix] [number_of_instances]

# Default values
PREFIX="ok"
INSTANCES=4

# Parse command line arguments
if [ $# -ge 1 ]; then
    PREFIX=$1
fi

if [ $# -ge 2 ]; then
    INSTANCES=$2
fi

echo "Running $INSTANCES instances of vanity-id-rust searching for prefix: $PREFIX"
echo "Each instance will run in single-threaded mode"

# Create a temporary directory for storing results
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
OUTPUT_DIR="results_$TIMESTAMP"
mkdir -p "$OUTPUT_DIR"

echo "Results will be stored in: $OUTPUT_DIR"

# Export the function and variables for GNU parallel
export PREFIX
export OUTPUT_DIR

# Function to run a single instance
run_instance() {
    local instance_id=$1
    local output_file="$OUTPUT_DIR/result_$instance_id.txt"
    
    echo "Starting instance $instance_id" >&2
    
    # Run the vanity-id-rust in single-threaded mode
    # We'll capture the output and also save the key file with a unique name
    cd /home/aagat/experiments/vanity-id/scratch
    cargo run --release -- --prefix "$PREFIX" --single-thread > "$output_file" 2>&1
    
    # If a key.pem was generated, rename it to include the instance ID
    if [ -f "key.pem" ]; then
        mv "key.pem" "$OUTPUT_DIR/key_$instance_id.pem"
        echo "Instance $instance_id found a match! Key saved to $OUTPUT_DIR/key_$instance_id.pem" >&2
    fi
    
    echo "Instance $instance_id completed" >&2
}

# Export the function for GNU parallel
export -f run_instance

# Run instances in parallel
echo "Starting parallel execution..."
seq 1 $INSTANCES | parallel -j $INSTANCES run_instance {}

echo "All instances completed. Check $OUTPUT_DIR for results."

# Check if any matches were found
MATCH_COUNT=$(ls $OUTPUT_DIR/key_*.pem 2>/dev/null | wc -l)
if [ $MATCH_COUNT -gt 0 ]; then
    echo "Found $MATCH_COUNT match(es):"
    ls $OUTPUT_DIR/key_*.pem
else
    echo "No matches found. Check the result files in $OUTPUT_DIR for details."
fi
