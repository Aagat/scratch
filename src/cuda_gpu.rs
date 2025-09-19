use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// FFI declarations for CUDA functions
extern "C" {
    fn cuda_init(
        max_threads_per_block: *mut c_int,
        device_name: *mut c_char,
        name_len: c_int,
    ) -> c_int;
    fn cuda_search_vanity_id(
        prefix: *const c_char,
        prefix_len: c_int,
        start_counter: u64,
        batch_size: u64,
        results: *mut u32,
    ) -> c_int;
    fn cuda_cleanup();
}

pub struct CudaVanityGenerator {
    max_threads_per_block: i32,
    device_name: String,
}

impl CudaVanityGenerator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut max_threads_per_block: c_int = 0;
        let mut device_name_buffer = [0u8; 256];

        // Initialize CUDA
        let result = unsafe {
            cuda_init(
                &mut max_threads_per_block,
                device_name_buffer.as_mut_ptr() as *mut c_char,
                device_name_buffer.len() as c_int,
            )
        };

        match result {
            0 => {
                // Success - extract device name
                let device_name = unsafe {
                    CStr::from_ptr(device_name_buffer.as_ptr() as *const c_char)
                        .to_string_lossy()
                        .into_owned()
                };

                println!("Using CUDA GPU: {}", device_name);

                Ok(CudaVanityGenerator {
                    max_threads_per_block,
                    device_name,
                })
            }
            -1 => {
                Err("No CUDA devices found. This requires an NVIDIA GPU with CUDA support.".into())
            }
            -2 => Err("Failed to get CUDA device properties.".into()),
            -3 => Err("Failed to set CUDA device.".into()),
            -4 => Err("Failed to allocate CUDA results buffer.".into()),
            -5 => Err("Failed to allocate CUDA prefix buffer.".into()),
            _ => Err(format!("Unknown CUDA initialization error: {}", result).into()),
        }
    }

    pub fn search_vanity_id(
        &self,
        prefix: &str,
        start_counter: u64,
        batch_size: u64,
    ) -> Result<Option<(u64, [u8; 32])>, Box<dyn std::error::Error>> {
        let prefix_cstring = CString::new(prefix)?;
        let prefix_len = prefix.len() as c_int;

        // Create results buffer: [found_flag, counter_low, counter_high, key_data_as_8_u32s]
        let mut results = [0u32; 11];

        let result = unsafe {
            cuda_search_vanity_id(
                prefix_cstring.as_ptr(),
                prefix_len,
                start_counter,
                batch_size,
                results.as_mut_ptr(),
            )
        };

        match result {
            0 => {
                // Success - check if we found a match
                let found_flag = results[0];

                if found_flag != 0 {
                    // Reconstruct 64-bit counter from two 32-bit values
                    let counter_low = results[1] as u64;
                    let counter_high = results[2] as u64;
                    let counter = counter_low | (counter_high << 32);

                    // Reconstruct key data from the 8 u32 chunks
                    let mut key_data = [0u8; 32];
                    for i in 0..8 {
                        let chunk = results[3 + i];
                        for j in 0..4 {
                            key_data[i * 4 + j] = ((chunk >> (j * 8)) & 0xFF) as u8;
                        }
                    }

                    return Ok(Some((counter, key_data)));
                }

                Ok(None)
            }
            -1 => Err("CUDA not initialized.".into()),
            -2 => Err("Failed to copy prefix to CUDA device.".into()),
            -3 => Err("Failed to initialize CUDA results buffer.".into()),
            -4 => Err("CUDA kernel execution failed.".into()),
            -5 => Err("Failed to copy results from CUDA device.".into()),
            _ => Err(format!("Unknown CUDA search error: {}", result).into()),
        }
    }

    pub fn get_max_threads_per_block(&self) -> usize {
        self.max_threads_per_block as usize
    }

    pub fn get_device_name(&self) -> String {
        self.device_name.clone()
    }
}

impl Drop for CudaVanityGenerator {
    fn drop(&mut self) {
        unsafe {
            cuda_cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cuda_initialization() {
        match CudaVanityGenerator::new() {
            Ok(gpu) => {
                println!(
                    "CUDA GPU initialized successfully: {}",
                    gpu.get_device_name()
                );
                println!("Max threads per block: {}", gpu.get_max_threads_per_block());
            }
            Err(e) => {
                println!("CUDA GPU initialization failed: {}", e);
                // This is expected on systems without NVIDIA GPUs or CUDA
            }
        }
    }

    #[test]
    fn test_cuda_search_small_batch() {
        if let Ok(gpu) = CudaVanityGenerator::new() {
            // Test with a very small batch to see if it works
            match gpu.search_vanity_id("a", 0, 1000) {
                Ok(result) => {
                    if let Some((counter, key_data)) = result {
                        println!("Found match at counter {}: {:?}", counter, key_data);
                    } else {
                        println!("No match found in small batch (expected)");
                    }
                }
                Err(e) => {
                    println!("CUDA search failed: {}", e);
                }
            }
        }
    }
}
