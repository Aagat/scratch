use metal::*;
use std::mem;

pub struct GpuVanityGenerator {
    device: Device,
    command_queue: CommandQueue,
    compute_pipeline: ComputePipelineState,
}

impl GpuVanityGenerator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Get the default Metal device (Apple Silicon GPU)
        let device = Device::system_default()
            .ok_or("No Metal device found. This requires Apple Silicon.")?;

        println!("Using GPU: {}", device.name());

        // Create command queue
        let command_queue = device.new_command_queue();

        // Load and compile the Metal shader
        let shader_source = include_str!("vanity_id.metal");
        let library = device
            .new_library_with_source(shader_source, &CompileOptions::new())
            .map_err(|e| format!("Failed to compile Metal shader: {}", e))?;

        // Get the compute function
        let function = library
            .get_function("vanity_search", None)
            .map_err(|e| format!("Failed to find vanity_search function in shader: {}", e))?;

        // Create compute pipeline
        let compute_pipeline = device
            .new_compute_pipeline_state_with_function(&function)
            .map_err(|e| format!("Failed to create compute pipeline: {}", e))?;

        Ok(GpuVanityGenerator {
            device,
            command_queue,
            compute_pipeline,
        })
    }

    pub fn search_vanity_id(
        &self,
        prefix: &str,
        start_counter: u64,
        batch_size: u64,
    ) -> Result<Option<(u64, [u8; 32])>, Box<dyn std::error::Error>> {
        let prefix_bytes = prefix.as_bytes();
        let prefix_len = prefix_bytes.len() as u32;

        // Create buffers
        let results_size = mem::size_of::<u32>() * 11; // [found_flag, counter_low, counter_high, key_data_as_8_u32s]
        let results_buffer = self
            .device
            .new_buffer(results_size as u64, MTLResourceOptions::StorageModeShared);

        let prefix_buffer = self.device.new_buffer_with_data(
            prefix_bytes.as_ptr() as *const _,
            prefix_bytes.len() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let prefix_len_buffer = self.device.new_buffer_with_data(
            &prefix_len as *const u32 as *const _,
            mem::size_of::<u32>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let start_counter_buffer = self.device.new_buffer_with_data(
            &start_counter as *const u64 as *const _,
            mem::size_of::<u64>() as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Initialize results buffer to zero
        unsafe {
            let results_ptr = results_buffer.contents() as *mut u32;
            for i in 0..11 {
                *results_ptr.add(i) = 0;
            }
        }

        // Create command buffer and encoder
        let command_buffer = self.command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();

        // Set compute pipeline and buffers
        encoder.set_compute_pipeline_state(&self.compute_pipeline);
        encoder.set_buffer(0, Some(&results_buffer), 0);
        encoder.set_buffer(1, Some(&prefix_buffer), 0);
        encoder.set_buffer(2, Some(&prefix_len_buffer), 0);
        encoder.set_buffer(3, Some(&start_counter_buffer), 0);

        // Calculate thread group sizes
        let max_threads_per_group = self.compute_pipeline.max_total_threads_per_threadgroup();
        let threads_per_group = std::cmp::min(max_threads_per_group, 256);
        let thread_groups = (batch_size + threads_per_group as u64 - 1) / threads_per_group as u64;

        // Dispatch threads
        encoder.dispatch_thread_groups(
            MTLSize::new(thread_groups, 1, 1),
            MTLSize::new(threads_per_group, 1, 1),
        );

        encoder.end_encoding();
        command_buffer.commit();
        command_buffer.wait_until_completed();

        // Check results
        unsafe {
            let results_ptr = results_buffer.contents() as *const u32;
            let found_flag = *results_ptr;

            if found_flag != 0 {
                // Reconstruct 64-bit counter from two 32-bit values
                let counter_low = *results_ptr.add(1) as u64;
                let counter_high = *results_ptr.add(2) as u64;
                let counter = counter_low | (counter_high << 32);

                // Reconstruct key data from the 8 u32 chunks
                let mut key_data = [0u8; 32];
                for i in 0..8 {
                    let chunk = *results_ptr.add(3 + i);
                    for j in 0..4 {
                        key_data[i * 4 + j] = ((chunk >> (j * 8)) & 0xFF) as u8;
                    }
                }

                return Ok(Some((counter, key_data)));
            }
        }

        Ok(None)
    }

    pub fn get_max_threads_per_group(&self) -> usize {
        self.compute_pipeline.max_total_threads_per_threadgroup() as usize
    }

    pub fn get_device_name(&self) -> String {
        self.device.name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_initialization() {
        match GpuVanityGenerator::new() {
            Ok(gpu) => {
                println!("GPU initialized successfully: {}", gpu.get_device_name());
                println!("Max threads per group: {}", gpu.get_max_threads_per_group());
            }
            Err(e) => {
                println!("GPU initialization failed: {}", e);
                // This is expected on non-Apple Silicon systems
            }
        }
    }

    #[test]
    fn test_gpu_search_small_batch() {
        if let Ok(gpu) = GpuVanityGenerator::new() {
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
                    println!("GPU search failed: {}", e);
                }
            }
        }
    }
}
