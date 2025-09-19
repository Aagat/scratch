#include <cuda_runtime.h>
#include <device_launch_parameters.h>
#include <stdint.h>

// Constants for the character mapping
__constant__ char MAPPING[16] = {'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p'};

// SHA-256 constants
__constant__ uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};

// SHA-256 helper functions
__device__ uint32_t rotr(uint32_t x, uint32_t n)
{
  return (x >> n) | (x << (32 - n));
}

__device__ uint32_t ch(uint32_t x, uint32_t y, uint32_t z)
{
  return (x & y) ^ (~x & z);
}

__device__ uint32_t maj(uint32_t x, uint32_t y, uint32_t z)
{
  return (x & y) ^ (x & z) ^ (y & z);
}

__device__ uint32_t sigma0(uint32_t x)
{
  return rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22);
}

__device__ uint32_t sigma1(uint32_t x)
{
  return rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25);
}

__device__ uint32_t gamma0(uint32_t x)
{
  return rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3);
}

__device__ uint32_t gamma1(uint32_t x)
{
  return rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10);
}

// Generate key data from counter
__device__ void generate_key_data(uint64_t counter, unsigned char *key_data)
{
  // Clear the array
  for (int i = 0; i < 32; i++)
  {
    key_data[i] = 0;
  }

  // Use counter as base (little endian)
  for (int i = 0; i < 8; i++)
  {
    key_data[i] = (counter >> (i * 8)) & 0xFF;
  }

  // Fill remaining bytes with derived values
  for (int i = 8; i < 32; i++)
  {
    key_data[i] = ((counter >> (i % 8)) ^ i) & 0xFF;
  }
}

// SHA-256 implementation
__device__ void sha256(const unsigned char *data, unsigned char *hash)
{
  uint32_t h[8] = {
      0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
      0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};

  // Prepare message schedule
  uint32_t w[64];

  // Copy input data to first 16 words (big endian)
  for (int i = 0; i < 8; i++)
  {
    w[i] = (data[i * 4] << 24) | (data[i * 4 + 1] << 16) | (data[i * 4 + 2] << 8) | data[i * 4 + 3];
  }

  // Padding: append 1 bit followed by zeros, then length
  w[8] = 0x80000000; // First padding bit
  for (int i = 9; i < 14; i++)
  {
    w[i] = 0;
  }
  w[14] = 0;   // High 32 bits of length (always 0 for our 32-byte input)
  w[15] = 256; // Low 32 bits of length (32 bytes = 256 bits)

  // Extend the first 16 words into the remaining 48 words
  for (int i = 16; i < 64; i++)
  {
    w[i] = gamma1(w[i - 2]) + w[i - 7] + gamma0(w[i - 15]) + w[i - 16];
  }

  // Main loop
  uint32_t a = h[0], b = h[1], c = h[2], d = h[3];
  uint32_t e = h[4], f = h[5], g = h[6], h_val = h[7];

  for (int i = 0; i < 64; i++)
  {
    uint32_t t1 = h_val + sigma1(e) + ch(e, f, g) + K[i] + w[i];
    uint32_t t2 = sigma0(a) + maj(a, b, c);
    h_val = g;
    g = f;
    f = e;
    e = d + t1;
    d = c;
    c = b;
    b = a;
    a = t1 + t2;
  }

  // Add this chunk's hash to result
  h[0] += a;
  h[1] += b;
  h[2] += c;
  h[3] += d;
  h[4] += e;
  h[5] += f;
  h[6] += g;
  h[7] += h_val;

  // Convert to bytes (big endian)
  for (int i = 0; i < 8; i++)
  {
    hash[i * 4] = (h[i] >> 24) & 0xFF;
    hash[i * 4 + 1] = (h[i] >> 16) & 0xFF;
    hash[i * 4 + 2] = (h[i] >> 8) & 0xFF;
    hash[i * 4 + 3] = h[i] & 0xFF;
  }
}

// Check if hash matches prefix
__device__ bool hash_matches_prefix(const unsigned char *hash, const unsigned char *prefix, uint32_t prefix_len)
{
  if (prefix_len == 0)
    return true;

  uint32_t full_bytes = prefix_len / 2;

  // Check full bytes
  for (uint32_t byte_idx = 0; byte_idx < full_bytes; byte_idx++)
  {
    unsigned char hash_byte = hash[byte_idx];
    unsigned char expected_high = prefix[byte_idx * 2];
    unsigned char expected_low = prefix[byte_idx * 2 + 1];

    unsigned char actual_high = MAPPING[(hash_byte >> 4) & 0x0F];
    unsigned char actual_low = MAPPING[hash_byte & 0x0F];

    if (actual_high != expected_high || actual_low != expected_low)
    {
      return false;
    }
  }

  // Handle odd-length prefix
  if (prefix_len % 2 == 1)
  {
    unsigned char hash_byte = hash[full_bytes];
    unsigned char expected_char = prefix[prefix_len - 1];
    unsigned char actual_char = MAPPING[(hash_byte >> 4) & 0x0F];

    if (actual_char != expected_char)
    {
      return false;
    }
  }

  return true;
}

// Main CUDA kernel
__global__ void vanity_search(
    uint32_t *results,           // Output: [found_flag, counter_low, counter_high, key_data...]
    const unsigned char *prefix, // Input: prefix to search for
    uint32_t prefix_len,         // Input: length of prefix
    uint64_t start_counter       // Input: starting counter value
)
{
  uint64_t thread_id = blockIdx.x * blockDim.x + threadIdx.x;
  uint64_t counter = start_counter + thread_id;

  // Generate key data
  unsigned char key_data[32];
  generate_key_data(counter, key_data);

  // Compute SHA-256 hash
  unsigned char hash[32];
  sha256(key_data, hash);

  // Check if it matches the prefix
  if (hash_matches_prefix(hash, prefix, prefix_len))
  {
    // Atomically set found flag
    uint32_t old = atomicCAS(&results[0], 0, 1);
    if (old == 0)
    {
      // We're the first to find a match, store the result
      // Split 64-bit counter into two 32-bit values
      results[1] = (uint32_t)(counter & 0xFFFFFFFF);         // Low 32 bits
      results[2] = (uint32_t)((counter >> 32) & 0xFFFFFFFF); // High 32 bits

      // Store key data (convert to uint32 for easier transfer)
      for (int i = 0; i < 8; i++)
      {
        uint32_t key_chunk = 0;
        for (int j = 0; j < 4; j++)
        {
          key_chunk |= ((uint32_t)key_data[i * 4 + j]) << (j * 8);
        }
        results[3 + i] = key_chunk;
      }
    }
  }
}

// C interface for Rust FFI
extern "C"
{
  // Initialize CUDA and return device properties
  int cuda_init(int *max_threads_per_block, char *device_name, int name_len);

  // Search for vanity ID using CUDA
  int cuda_search_vanity_id(
      const char *prefix,
      int prefix_len,
      uint64_t start_counter,
      uint64_t batch_size,
      uint32_t *results // [found_flag, counter_low, counter_high, key_data_as_8_u32s]
  );

  // Cleanup CUDA resources
  void cuda_cleanup();
}

// Global device pointers
static unsigned char *d_prefix = nullptr;
static uint32_t *d_results = nullptr;
static bool cuda_initialized = false;

int cuda_init(int *max_threads_per_block, char *device_name, int name_len)
{
  if (cuda_initialized)
  {
    return 0; // Already initialized
  }

  // Check for CUDA devices
  int device_count;
  cudaError_t error = cudaGetDeviceCount(&device_count);
  if (error != cudaSuccess || device_count == 0)
  {
    return -1; // No CUDA devices found
  }

  // Get device properties
  cudaDeviceProp prop;
  error = cudaGetDeviceProperties(&prop, 0);
  if (error != cudaSuccess)
  {
    return -2; // Failed to get device properties
  }

  // Set device
  error = cudaSetDevice(0);
  if (error != cudaSuccess)
  {
    return -3; // Failed to set device
  }

  // Copy device name
  int copy_len = (name_len - 1 < strlen(prop.name)) ? name_len - 1 : strlen(prop.name);
  strncpy(device_name, prop.name, copy_len);
  device_name[copy_len] = '\0';

  *max_threads_per_block = prop.maxThreadsPerBlock;

  // Allocate device memory for results (11 uint32_t values)
  error = cudaMalloc(&d_results, 11 * sizeof(uint32_t));
  if (error != cudaSuccess)
  {
    return -4; // Failed to allocate results buffer
  }

  // Allocate device memory for prefix (max 64 characters should be enough)
  error = cudaMalloc(&d_prefix, 64);
  if (error != cudaSuccess)
  {
    cudaFree(d_results);
    return -5; // Failed to allocate prefix buffer
  }

  cuda_initialized = true;
  return 0; // Success
}

int cuda_search_vanity_id(
    const char *prefix,
    int prefix_len,
    uint64_t start_counter,
    uint64_t batch_size,
    uint32_t *results)
{
  if (!cuda_initialized)
  {
    return -1; // CUDA not initialized
  }

  // Copy prefix to device
  cudaError_t error = cudaMemcpy(d_prefix, prefix, prefix_len, cudaMemcpyHostToDevice);
  if (error != cudaSuccess)
  {
    return -2; // Failed to copy prefix
  }

  // Initialize results buffer to zero
  error = cudaMemset(d_results, 0, 11 * sizeof(uint32_t));
  if (error != cudaSuccess)
  {
    return -3; // Failed to initialize results
  }

  // Calculate grid and block dimensions
  int threads_per_block = 256; // Good default for most GPUs
  int blocks = (batch_size + threads_per_block - 1) / threads_per_block;

  // Limit the number of blocks to avoid excessive GPU usage
  const int max_blocks = 65535; // CUDA grid limit
  if (blocks > max_blocks)
  {
    blocks = max_blocks;
  }

  // Launch kernel
  vanity_search<<<blocks, threads_per_block>>>(
      d_results,
      d_prefix,
      prefix_len,
      start_counter);

  // Wait for kernel to complete
  error = cudaDeviceSynchronize();
  if (error != cudaSuccess)
  {
    return -4; // Kernel execution failed
  }

  // Copy results back to host
  error = cudaMemcpy(results, d_results, 11 * sizeof(uint32_t), cudaMemcpyDeviceToHost);
  if (error != cudaSuccess)
  {
    return -5; // Failed to copy results
  }

  return 0; // Success
}

void cuda_cleanup()
{
  if (cuda_initialized)
  {
    if (d_prefix)
    {
      cudaFree(d_prefix);
      d_prefix = nullptr;
    }
    if (d_results)
    {
      cudaFree(d_results);
      d_results = nullptr;
    }
    cudaDeviceReset();
    cuda_initialized = false;
  }
}
