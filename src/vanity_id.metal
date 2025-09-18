#include <metal_stdlib>
using namespace metal;

// Constants for the character mapping
constant char MAPPING[16] = {'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p'};

// SHA-256 constants
constant uint K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

// SHA-256 helper functions
uint rotr(uint x, uint n) {
    return (x >> n) | (x << (32 - n));
}

uint ch(uint x, uint y, uint z) {
    return (x & y) ^ (~x & z);
}

uint maj(uint x, uint y, uint z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

uint sigma0(uint x) {
    return rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22);
}

uint sigma1(uint x) {
    return rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25);
}

uint gamma0(uint x) {
    return rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3);
}

uint gamma1(uint x) {
    return rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10);
}

// Generate key data from counter
void generate_key_data(uint64_t counter, thread uchar* key_data) {
    // Clear the array
    for (int i = 0; i < 32; i++) {
        key_data[i] = 0;
    }
    
    // Use counter as base (little endian)
    for (int i = 0; i < 8; i++) {
        key_data[i] = (counter >> (i * 8)) & 0xFF;
    }
    
    // Fill remaining bytes with derived values
    for (int i = 8; i < 32; i++) {
        key_data[i] = ((counter >> (i % 8)) ^ i) & 0xFF;
    }
}

// SHA-256 implementation
void sha256(thread const uchar* data, thread uchar* hash) {
    uint h[8] = {
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    };
    
    // Prepare message schedule
    uint w[64];
    
    // Copy input data to first 16 words (big endian)
    for (int i = 0; i < 8; i++) {
        w[i] = (data[i*4] << 24) | (data[i*4+1] << 16) | (data[i*4+2] << 8) | data[i*4+3];
    }
    
    // Padding: append 1 bit followed by zeros, then length
    w[8] = 0x80000000; // First padding bit
    for (int i = 9; i < 14; i++) {
        w[i] = 0;
    }
    w[14] = 0; // High 32 bits of length (always 0 for our 32-byte input)
    w[15] = 256; // Low 32 bits of length (32 bytes = 256 bits)
    
    // Extend the first 16 words into the remaining 48 words
    for (int i = 16; i < 64; i++) {
        w[i] = gamma1(w[i-2]) + w[i-7] + gamma0(w[i-15]) + w[i-16];
    }
    
    // Main loop
    uint a = h[0], b = h[1], c = h[2], d = h[3];
    uint e = h[4], f = h[5], g = h[6], h_val = h[7];
    
    for (int i = 0; i < 64; i++) {
        uint t1 = h_val + sigma1(e) + ch(e, f, g) + K[i] + w[i];
        uint t2 = sigma0(a) + maj(a, b, c);
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
    h[0] += a; h[1] += b; h[2] += c; h[3] += d;
    h[4] += e; h[5] += f; h[6] += g; h[7] += h_val;
    
    // Convert to bytes (big endian)
    for (int i = 0; i < 8; i++) {
        hash[i*4] = (h[i] >> 24) & 0xFF;
        hash[i*4+1] = (h[i] >> 16) & 0xFF;
        hash[i*4+2] = (h[i] >> 8) & 0xFF;
        hash[i*4+3] = h[i] & 0xFF;
    }
}

// Check if hash matches prefix
bool hash_matches_prefix(thread const uchar* hash, constant uchar* prefix, uint prefix_len) {
    if (prefix_len == 0) return true;
    
    uint full_bytes = prefix_len / 2;
    
    // Check full bytes
    for (uint byte_idx = 0; byte_idx < full_bytes; byte_idx++) {
        uchar hash_byte = hash[byte_idx];
        uchar expected_high = prefix[byte_idx * 2];
        uchar expected_low = prefix[byte_idx * 2 + 1];
        
        uchar actual_high = MAPPING[(hash_byte >> 4) & 0x0F];
        uchar actual_low = MAPPING[hash_byte & 0x0F];
        
        if (actual_high != expected_high || actual_low != expected_low) {
            return false;
        }
    }
    
    // Handle odd-length prefix
    if (prefix_len % 2 == 1) {
        uchar hash_byte = hash[full_bytes];
        uchar expected_char = prefix[prefix_len - 1];
        uchar actual_char = MAPPING[(hash_byte >> 4) & 0x0F];
        
        if (actual_char != expected_char) {
            return false;
        }
    }
    
    return true;
}

// Main compute kernel
kernel void vanity_search(
    device uint* results [[buffer(0)]],            // Output: [found_flag, counter_low, counter_high, key_data...]
    constant uchar* prefix [[buffer(1)]],          // Input: prefix to search for
    constant uint& prefix_len [[buffer(2)]],       // Input: length of prefix
    constant uint64_t& start_counter [[buffer(3)]], // Input: starting counter value
    uint gid [[thread_position_in_grid]]
) {
    uint64_t counter = start_counter + gid;
    
    // Generate key data
    uchar key_data[32];
    generate_key_data(counter, key_data);
    
    // Compute SHA-256 hash
    uchar hash[32];
    sha256(key_data, hash);
    
    // Check if it matches the prefix
    if (hash_matches_prefix(hash, prefix, prefix_len)) {
        // Atomically set found flag using 32-bit atomic
        uint expected = 0;
        if (atomic_compare_exchange_weak_explicit(
            (device atomic_uint*)&results[0], 
            &expected, 
            1, 
            memory_order_relaxed, 
            memory_order_relaxed)) {
            
            // We're the first to find a match, store the result
            // Split 64-bit counter into two 32-bit values
            results[1] = (uint)(counter & 0xFFFFFFFF);        // Low 32 bits
            results[2] = (uint)((counter >> 32) & 0xFFFFFFFF); // High 32 bits
            
            // Store key data (convert to uint32 for easier transfer)
            for (int i = 0; i < 8; i++) {
                uint key_chunk = 0;
                for (int j = 0; j < 4; j++) {
                    key_chunk |= ((uint)key_data[i*4 + j]) << (j * 8);
                }
                results[3 + i] = key_chunk;
            }
        }
    }
}
