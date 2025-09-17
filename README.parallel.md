# Parallel Vanity ID Generator

This script allows you to run multiple instances of the vanity-id-rust tool in single-threaded mode simultaneously using GNU parallel.

## Usage

```bash
./run_parallel.sh [prefix] [number_of_instances]
```

- `prefix`: The desired prefix for the extension ID (default: "ok")
- `number_of_instances`: Number of parallel instances to run (default: 4)

## Examples

```bash
# Run 4 instances searching for IDs starting with "abc"
./run_parallel.sh abc 4

# Run 8 instances searching for IDs starting with "test"
./run_parallel.sh test 8

# Run with default values (prefix "ok", 4 instances)
./run_parallel.sh
```

## Output

The script will create a timestamped directory (e.g., `results_20250101_120000`) containing:
- `result_*.txt`: Output from each instance
- `public_key.der`: Public key in DER format for any matches found
- `public_key.pem`: Public key in PEM format for any matches found

Additionally, the base64-encoded public key will be printed in the output which can be directly copied into your Chrome extension's manifest.json file.

## How it works

Each instance runs the vanity-id-rust tool in single-threaded mode (`--single-thread` flag) to:
1. Distribute the search across multiple processes
2. Avoid oversubscribing CPU cores
3. Increase the chances of finding a match faster

The script requires GNU parallel to be installed on the system.