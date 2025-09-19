use base64;
use base64::Engine;
use clap::Parser;
use num_format::{Locale, ToFormattedString};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[cfg(target_os = "macos")]
mod gpu;
#[cfg(target_os = "macos")]
use gpu::GpuVanityGenerator;

#[cfg(feature = "cuda")]
mod cuda_gpu;
#[cfg(feature = "cuda")]
use cuda_gpu::CudaVanityGenerator;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ok")]
    prefix: String,

    #[arg(short, long, default_value_t = num_cpus::get())]
    cores: usize,

    #[arg(long)]
    single_thread: bool,

    #[arg(
        long,
        help = "Use GPU acceleration (Metal on macOS, CUDA on Windows/Linux)"
    )]
    gpu: bool,

    #[arg(
        long,
        help = "Use both GPU and CPU simultaneously for maximum performance"
    )]
    hybrid: bool,

    #[arg(
        long,
        default_value_t = 1_000_000,
        help = "GPU batch size for each compute dispatch"
    )]
    gpu_batch_size: u64,
}

const MAPPING: [char; 16] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
];

fn main() {
    let cli = Cli::parse();

    if cli.hybrid {
        println!("Searching for extension ID with prefix: {}", cli.prefix);
        println!("Using hybrid mode: GPU + CPU simultaneously");
        // Use fewer CPU threads in hybrid mode to avoid resource contention with GPU
        let cpu_threads = if cli.single_thread {
            1
        } else {
            std::cmp::max(1, cli.cores / 4)
        };
        #[cfg(target_os = "macos")]
        run_hybrid_vanity_id_generator(&cli.prefix, cpu_threads, cli.gpu_batch_size);
        #[cfg(not(target_os = "macos"))]
        run_cuda_hybrid_vanity_id_generator(&cli.prefix, cpu_threads, cli.gpu_batch_size);
        return;
    }

    if cli.gpu {
        println!("Searching for extension ID with prefix: {}", cli.prefix);
        #[cfg(target_os = "macos")]
        {
            println!("Using GPU acceleration (Metal - Apple Silicon)");
            run_gpu_vanity_id_generator(&cli.prefix, cli.gpu_batch_size);
        }
        #[cfg(not(target_os = "macos"))]
        {
            println!("Using GPU acceleration (CUDA - NVIDIA)");
            run_cuda_gpu_vanity_id_generator(&cli.prefix, cli.gpu_batch_size);
        }
        return;
    }

    let thread_count = if cli.single_thread { 1 } else { cli.cores };
    println!("Searching for extension ID with prefix: {}", cli.prefix);
    println!("Using {} thread(s)", thread_count);

    run_vanity_id_generator(&cli.prefix, thread_count);
}

fn run_vanity_id_generator(desired_prefix: &str, num_threads: usize) {
    let start_time = Instant::now();
    let found = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));
    let last_progress_time = Arc::new(Mutex::new(Instant::now()));

    // Shared progress tracking - each thread will report its attempts
    let thread_attempts = Arc::new(Mutex::new(vec![0u64; num_threads]));

    // Calculate counter ranges for each thread to avoid overlap
    // Each thread gets a large range to work with independently
    const THREAD_RANGE_SIZE: u64 = u64::MAX / 1024; // Large range per thread

    // Spawn worker threads
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let prefix = desired_prefix.to_string();
            let found = Arc::clone(&found);
            let result = Arc::clone(&result);
            let start_time = start_time.clone();
            let last_progress_time = Arc::clone(&last_progress_time);
            let thread_attempts = Arc::clone(&thread_attempts);

            thread::spawn(move || {
                // Each thread calculates its own starting counter to avoid overlap
                let thread_start_counter = (thread_id as u64) * THREAD_RANGE_SIZE;
                let mut local_counter = thread_start_counter;
                let mut local_attempts = 0u64;
                const PROGRESS_REPORT_INTERVAL: u64 = 500000; // Report progress every 500k attempts in hybrid mode

                while !found.load(Ordering::Relaxed) {
                    if let Some((ext_id, key_data)) =
                        try_generate_match_optimized(&prefix, local_counter)
                    {
                        if !found.swap(true, Ordering::Relaxed) {
                            *result.lock().unwrap() = Some((ext_id, key_data, local_attempts + 1));
                        }
                        break;
                    }

                    local_counter += 1;
                    local_attempts += 1;

                    // Periodically update shared progress and print status (only thread 0)
                    if local_attempts % PROGRESS_REPORT_INTERVAL == 0 {
                        // Update this thread's attempt count
                        {
                            let mut attempts = thread_attempts.lock().unwrap();
                            attempts[thread_id] = local_attempts;
                        }

                        // Only thread 0 prints progress every second
                        if thread_id == 0 {
                            let now = Instant::now();
                            let mut last_time = last_progress_time.lock().unwrap();
                            if now.duration_since(*last_time).as_secs() >= 1 {
                                *last_time = now;

                                // Calculate total attempts across all threads
                                let total = {
                                    let attempts = thread_attempts.lock().unwrap();
                                    attempts.iter().sum::<u64>()
                                };

                                let elapsed = start_time.elapsed().as_secs_f64();
                                if elapsed > 0.0 {
                                    let rate = total as f64 / elapsed;
                                    println!(
                                        "Progress: {} attempts, {} keys/sec",
                                        total.to_formatted_string(&Locale::en),
                                        (rate as u64).to_formatted_string(&Locale::en)
                                    );
                                }
                            }
                        }
                    }
                }

                // Final update of this thread's attempts
                {
                    let mut attempts = thread_attempts.lock().unwrap();
                    attempts[thread_id] = local_attempts;
                }
            })
        })
        .collect();

    // Wait for completion
    for handle in handles {
        handle.join().unwrap();
    }

    // Output results
    let result_data = result.lock().unwrap().take();
    if let Some((ext_id, key_data, _winning_thread_attempts)) = result_data {
        let duration = start_time.elapsed().as_secs_f64();

        // Calculate total attempts across all threads
        let total = {
            let attempts = thread_attempts.lock().unwrap();
            attempts.iter().sum::<u64>()
        };

        let rate = total as f64 / duration;

        println!("\n🎉 Match found!");
        println!("Extension ID: {}", ext_id);
        println!("Total attempts: {}", total.to_formatted_string(&Locale::en));
        println!("Duration: {:.2} seconds", duration);
        println!(
            "Rate: {} keys/second",
            (rate as u64).to_formatted_string(&Locale::en)
        );

        // Save files
        save_key_files(&key_data);

        // Print base64 for manifest
        let base64_key = base64::engine::general_purpose::STANDARD.encode(&key_data);
        println!("\nPublic key for manifest.json:");
        println!("{}", base64_key);
    }
}

fn try_generate_match_optimized(desired_prefix: &str, counter: u64) -> Option<(String, [u8; 32])> {
    // Generate key data from counter
    let key_data = generate_key_data(counter);

    // Hash the key data
    let hash = Sha256::digest(&key_data);

    // Optimized hash matching with early exit
    if hash_matches_prefix_optimized(&hash, desired_prefix) {
        let extension_id = hash_to_extension_id(&hash);
        Some((extension_id, key_data))
    } else {
        None
    }
}

fn generate_key_data(counter: u64) -> [u8; 32] {
    let mut data = [0u8; 32];

    // Use counter as base
    let counter_bytes = counter.to_le_bytes();
    data[..8].copy_from_slice(&counter_bytes);

    // Fill remaining bytes with derived values
    for i in 8..32 {
        data[i] = ((counter >> (i % 8)) ^ (i as u64)) as u8;
    }

    data
}

fn hash_matches_prefix_optimized(hash: &[u8], prefix: &str) -> bool {
    let prefix_bytes = prefix.as_bytes();
    let prefix_len = prefix_bytes.len();

    // Early exit for empty prefix
    if prefix_len == 0 {
        return true;
    }

    // Process pairs of characters (full bytes) first for better performance
    let full_bytes = prefix_len / 2;
    for byte_idx in 0..full_bytes {
        let hash_byte = hash[byte_idx];
        let expected_high = prefix_bytes[byte_idx * 2];
        let expected_low = prefix_bytes[byte_idx * 2 + 1];

        // Convert hash byte to characters
        let actual_high = MAPPING[(hash_byte >> 4) as usize] as u8;
        let actual_low = MAPPING[(hash_byte & 0x0F) as usize] as u8;

        // Early exit on first mismatch
        if actual_high != expected_high || actual_low != expected_low {
            return false;
        }
    }

    // Handle odd-length prefix (remaining single character)
    if prefix_len % 2 == 1 {
        let hash_byte = hash[full_bytes];
        let expected_char = prefix_bytes[prefix_len - 1];
        let actual_char = MAPPING[(hash_byte >> 4) as usize] as u8;

        if actual_char != expected_char {
            return false;
        }
    }

    true
}

fn hash_to_extension_id(hash: &[u8]) -> String {
    hash[..16]
        .iter()
        .flat_map(|&byte| {
            let high = MAPPING[(byte >> 4) as usize];
            let low = MAPPING[(byte & 0x0F) as usize];
            [high, low]
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn run_hybrid_vanity_id_generator(
    desired_prefix: &str,
    num_cpu_threads: usize,
    gpu_batch_size: u64,
) {
    let start_time = Instant::now();
    let found = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));
    let last_progress_time = Arc::new(Mutex::new(Instant::now()));

    // Initialize GPU
    let gpu = match GpuVanityGenerator::new() {
        Ok(gpu) => {
            println!("GPU Device: {}", gpu.get_device_name());
            println!("Max threads per group: {}", gpu.get_max_threads_per_group());
            println!(
                "GPU batch size: {}",
                gpu_batch_size.to_formatted_string(&Locale::en)
            );
            println!("CPU threads: {}", num_cpu_threads);
            Some(gpu)
        }
        Err(e) => {
            eprintln!("Failed to initialize GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Shared progress tracking for both GPU and CPU
    let gpu_attempts = Arc::new(Mutex::new(0u64));
    let cpu_attempts = Arc::new(Mutex::new(vec![0u64; num_cpu_threads]));

    // Counter range allocation:
    // GPU gets the first half of the counter space (0 to u64::MAX/2)
    // CPU threads get the second half (u64::MAX/2 to u64::MAX)
    const GPU_RANGE_START: u64 = 0;
    const CPU_RANGE_START: u64 = u64::MAX / 2;
    const CPU_THREAD_RANGE_SIZE: u64 = (u64::MAX / 2) / 1024; // CPU threads share second half

    // Spawn GPU thread
    let gpu_handle = {
        let prefix = desired_prefix.to_string();
        let found = Arc::clone(&found);
        let result = Arc::clone(&result);
        let gpu_attempts = Arc::clone(&gpu_attempts);
        let gpu = gpu.unwrap();

        thread::spawn(move || {
            let mut batch_id = 0u64;
            let mut local_gpu_attempts = 0u64;

            while !found.load(Ordering::Relaxed) {
                // Calculate starting counter for this GPU batch
                let batch_start_counter = GPU_RANGE_START + (batch_id * gpu_batch_size);

                match gpu.search_vanity_id(&prefix, batch_start_counter, gpu_batch_size) {
                    Ok(Some((found_counter, key_data))) => {
                        // GPU found a match!
                        local_gpu_attempts += found_counter - batch_start_counter + 1;

                        if !found.swap(true, Ordering::Relaxed) {
                            *result.lock().unwrap() = Some((
                                hash_to_extension_id(&Sha256::digest(&key_data)),
                                key_data,
                                local_gpu_attempts,
                                "GPU".to_string(),
                            ));
                        }
                        break;
                    }
                    Ok(None) => {
                        // No match in this batch, continue
                        local_gpu_attempts += gpu_batch_size;
                        batch_id += 1;

                        // Update shared GPU attempts counter
                        *gpu_attempts.lock().unwrap() = local_gpu_attempts;
                    }
                    Err(e) => {
                        eprintln!("GPU error: {}", e);
                        break;
                    }
                }
            }

            // Final update
            *gpu_attempts.lock().unwrap() = local_gpu_attempts;
        })
    };

    // Spawn CPU threads
    let cpu_handles: Vec<_> = (0..num_cpu_threads)
        .map(|thread_id| {
            let prefix = desired_prefix.to_string();
            let found = Arc::clone(&found);
            let result = Arc::clone(&result);
            let start_time = start_time.clone();
            let last_progress_time = Arc::clone(&last_progress_time);
            let cpu_attempts = Arc::clone(&cpu_attempts);
            let gpu_attempts = Arc::clone(&gpu_attempts);

            thread::spawn(move || {
                // Each CPU thread gets a range in the second half of counter space
                let thread_start_counter =
                    CPU_RANGE_START + ((thread_id as u64) * CPU_THREAD_RANGE_SIZE);
                let mut local_counter = thread_start_counter;
                let mut local_attempts = 0u64;
                const PROGRESS_REPORT_INTERVAL: u64 = 500000; // Report progress every 500k attempts in hybrid mode

                while !found.load(Ordering::Relaxed) {
                    if let Some((ext_id, key_data)) =
                        try_generate_match_optimized(&prefix, local_counter)
                    {
                        if !found.swap(true, Ordering::Relaxed) {
                            *result.lock().unwrap() = Some((
                                ext_id,
                                key_data,
                                local_attempts + 1,
                                format!("CPU-{}", thread_id),
                            ));
                        }
                        break;
                    }

                    local_counter += 1;
                    local_attempts += 1;

                    // Periodically update shared progress and print status (only thread 0)
                    if local_attempts % PROGRESS_REPORT_INTERVAL == 0 {
                        // Update this CPU thread's attempt count
                        {
                            let mut attempts = cpu_attempts.lock().unwrap();
                            attempts[thread_id] = local_attempts;
                        }

                        // Only CPU thread 0 prints progress every second
                        if thread_id == 0 {
                            let now = Instant::now();
                            let mut last_time = last_progress_time.lock().unwrap();
                            if now.duration_since(*last_time).as_secs() >= 1 {
                                *last_time = now;

                                // Calculate total attempts across GPU and all CPU threads
                                let gpu_total = *gpu_attempts.lock().unwrap();
                                let cpu_total = {
                                    let attempts = cpu_attempts.lock().unwrap();
                                    attempts.iter().sum::<u64>()
                                };
                                let total = gpu_total + cpu_total;

                                let elapsed = start_time.elapsed().as_secs_f64();
                                if elapsed > 0.0 {
                                    let rate = total as f64 / elapsed;
                                    println!(
                                        "Progress: {} attempts (GPU: {}, CPU: {}), {} keys/sec",
                                        total.to_formatted_string(&Locale::en),
                                        gpu_total.to_formatted_string(&Locale::en),
                                        cpu_total.to_formatted_string(&Locale::en),
                                        (rate as u64).to_formatted_string(&Locale::en)
                                    );
                                }
                            }
                        }
                    }
                }

                // Final update of this CPU thread's attempts
                {
                    let mut attempts = cpu_attempts.lock().unwrap();
                    attempts[thread_id] = local_attempts;
                }
            })
        })
        .collect();

    // Wait for completion (either GPU or CPU finds a match)
    gpu_handle.join().unwrap();
    for handle in cpu_handles {
        handle.join().unwrap();
    }

    // Output results
    let result_data = result.lock().unwrap().take();
    if let Some((ext_id, key_data, _winning_attempts, winner)) = result_data {
        let duration = start_time.elapsed().as_secs_f64();

        // Calculate total attempts across GPU and all CPU threads
        let gpu_total = *gpu_attempts.lock().unwrap();
        let cpu_total = {
            let attempts = cpu_attempts.lock().unwrap();
            attempts.iter().sum::<u64>()
        };
        let total = gpu_total + cpu_total;

        let rate = total as f64 / duration;

        println!("\n🎉 Match found by {}!", winner);
        println!("Extension ID: {}", ext_id);
        println!(
            "Total attempts: {} (GPU: {}, CPU: {})",
            total.to_formatted_string(&Locale::en),
            gpu_total.to_formatted_string(&Locale::en),
            cpu_total.to_formatted_string(&Locale::en)
        );
        println!("Duration: {:.2} seconds", duration);
        println!(
            "Rate: {} keys/second",
            (rate as u64).to_formatted_string(&Locale::en)
        );

        // Save files
        save_key_files(&key_data);

        // Print base64 for manifest
        let base64_key = base64::engine::general_purpose::STANDARD.encode(&key_data);
        println!("\nPublic key for manifest.json:");
        println!("{}", base64_key);
    }
}

#[cfg(target_os = "macos")]
fn run_gpu_vanity_id_generator(desired_prefix: &str, batch_size: u64) {
    let start_time = Instant::now();
    let mut total_attempts = 0u64;
    let mut last_progress_time = Instant::now();

    // Initialize GPU
    let gpu = match GpuVanityGenerator::new() {
        Ok(gpu) => {
            println!("GPU Device: {}", gpu.get_device_name());
            println!("Max threads per group: {}", gpu.get_max_threads_per_group());
            println!(
                "Batch size: {}",
                batch_size.to_formatted_string(&Locale::en)
            );
            gpu
        }
        Err(e) => {
            eprintln!("Failed to initialize GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Use independent counter ranges like CPU implementation
    // Each batch gets a unique range to avoid overlap with other potential GPU instances
    let mut batch_id = 0u64;

    loop {
        // Calculate starting counter for this batch using independent ranges
        let batch_start_counter = batch_id * batch_size;

        match gpu.search_vanity_id(desired_prefix, batch_start_counter, batch_size) {
            Ok(Some((found_counter, key_data))) => {
                // Found a match!
                total_attempts += found_counter - batch_start_counter + 1;

                let duration = start_time.elapsed().as_secs_f64();
                let rate = total_attempts as f64 / duration;

                // Generate extension ID for display
                let hash = Sha256::digest(&key_data);
                let extension_id = hash_to_extension_id(&hash);

                println!("\n🎉 Match found!");
                println!("Extension ID: {}", extension_id);
                println!(
                    "Total attempts: {}",
                    total_attempts.to_formatted_string(&Locale::en)
                );
                println!("Duration: {:.2} seconds", duration);
                println!(
                    "Rate: {} keys/second",
                    (rate as u64).to_formatted_string(&Locale::en)
                );

                // Save files
                save_key_files(&key_data);

                // Print base64 for manifest
                let base64_key = base64::engine::general_purpose::STANDARD.encode(&key_data);
                println!("\nPublic key for manifest.json:");
                println!("{}", base64_key);
                break;
            }
            Ok(None) => {
                // No match in this batch, continue with next batch
                total_attempts += batch_size;
                batch_id += 1;

                // Print progress every second
                let now = Instant::now();
                if now.duration_since(last_progress_time).as_secs() >= 1 {
                    last_progress_time = now;
                    let elapsed = start_time.elapsed().as_secs_f64();
                    if elapsed > 0.0 {
                        let rate = total_attempts as f64 / elapsed;
                        println!(
                            "Progress: {} attempts, {} keys/sec",
                            total_attempts.to_formatted_string(&Locale::en),
                            (rate as u64).to_formatted_string(&Locale::en)
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("GPU error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

#[cfg(feature = "cuda")]
fn run_cuda_hybrid_vanity_id_generator(
    desired_prefix: &str,
    num_cpu_threads: usize,
    gpu_batch_size: u64,
) {
    let start_time = Instant::now();
    let found = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));
    let last_progress_time = Arc::new(Mutex::new(Instant::now()));

    // Initialize CUDA GPU
    let gpu = match CudaVanityGenerator::new() {
        Ok(gpu) => {
            println!("CUDA GPU Device: {}", gpu.get_device_name());
            println!("Max threads per block: {}", gpu.get_max_threads_per_block());
            println!(
                "CUDA GPU batch size: {}",
                gpu_batch_size.to_formatted_string(&Locale::en)
            );
            println!("CPU threads: {}", num_cpu_threads);
            Some(gpu)
        }
        Err(e) => {
            eprintln!("Failed to initialize CUDA GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Shared progress tracking for both GPU and CPU
    let gpu_attempts = Arc::new(Mutex::new(0u64));
    let cpu_attempts = Arc::new(Mutex::new(vec![0u64; num_cpu_threads]));

    // Counter range allocation:
    // GPU gets the first half of the counter space (0 to u64::MAX/2)
    // CPU threads get the second half (u64::MAX/2 to u64::MAX)
    const GPU_RANGE_START: u64 = 0;
    const CPU_RANGE_START: u64 = u64::MAX / 2;
    const CPU_THREAD_RANGE_SIZE: u64 = (u64::MAX / 2) / 1024; // CPU threads share second half

    // Spawn GPU thread
    let gpu_handle = {
        let prefix = desired_prefix.to_string();
        let found = Arc::clone(&found);
        let result = Arc::clone(&result);
        let gpu_attempts = Arc::clone(&gpu_attempts);
        let gpu = gpu.unwrap();

        thread::spawn(move || {
            let mut batch_id = 0u64;
            let mut local_gpu_attempts = 0u64;

            while !found.load(Ordering::Relaxed) {
                // Calculate starting counter for this GPU batch
                let batch_start_counter = GPU_RANGE_START + (batch_id * gpu_batch_size);

                match gpu.search_vanity_id(&prefix, batch_start_counter, gpu_batch_size) {
                    Ok(Some((found_counter, key_data))) => {
                        // GPU found a match!
                        local_gpu_attempts += found_counter - batch_start_counter + 1;

                        if !found.swap(true, Ordering::Relaxed) {
                            *result.lock().unwrap() = Some((
                                hash_to_extension_id(&Sha256::digest(&key_data)),
                                key_data,
                                local_gpu_attempts,
                                "CUDA GPU".to_string(),
                            ));
                        }
                        break;
                    }
                    Ok(None) => {
                        // No match in this batch, continue
                        local_gpu_attempts += gpu_batch_size;
                        batch_id += 1;

                        // Update shared GPU attempts counter
                        *gpu_attempts.lock().unwrap() = local_gpu_attempts;
                    }
                    Err(e) => {
                        eprintln!("CUDA GPU error: {}", e);
                        break;
                    }
                }
            }

            // Final update
            *gpu_attempts.lock().unwrap() = local_gpu_attempts;
        })
    };

    // Spawn CPU threads (same as Metal hybrid implementation)
    let cpu_handles: Vec<_> = (0..num_cpu_threads)
        .map(|thread_id| {
            let prefix = desired_prefix.to_string();
            let found = Arc::clone(&found);
            let result = Arc::clone(&result);
            let start_time = start_time.clone();
            let last_progress_time = Arc::clone(&last_progress_time);
            let cpu_attempts = Arc::clone(&cpu_attempts);
            let gpu_attempts = Arc::clone(&gpu_attempts);

            thread::spawn(move || {
                // Each CPU thread gets a range in the second half of counter space
                let thread_start_counter =
                    CPU_RANGE_START + ((thread_id as u64) * CPU_THREAD_RANGE_SIZE);
                let mut local_counter = thread_start_counter;
                let mut local_attempts = 0u64;
                const PROGRESS_REPORT_INTERVAL: u64 = 500000; // Report progress every 500k attempts in hybrid mode

                while !found.load(Ordering::Relaxed) {
                    if let Some((ext_id, key_data)) =
                        try_generate_match_optimized(&prefix, local_counter)
                    {
                        if !found.swap(true, Ordering::Relaxed) {
                            *result.lock().unwrap() = Some((
                                ext_id,
                                key_data,
                                local_attempts + 1,
                                format!("CPU-{}", thread_id),
                            ));
                        }
                        break;
                    }

                    local_counter += 1;
                    local_attempts += 1;

                    // Periodically update shared progress and print status (only thread 0)
                    if local_attempts % PROGRESS_REPORT_INTERVAL == 0 {
                        // Update this CPU thread's attempt count
                        {
                            let mut attempts = cpu_attempts.lock().unwrap();
                            attempts[thread_id] = local_attempts;
                        }

                        // Only CPU thread 0 prints progress every second
                        if thread_id == 0 {
                            let now = Instant::now();
                            let mut last_time = last_progress_time.lock().unwrap();
                            if now.duration_since(*last_time).as_secs() >= 1 {
                                *last_time = now;

                                // Calculate total attempts across GPU and all CPU threads
                                let gpu_total = *gpu_attempts.lock().unwrap();
                                let cpu_total = {
                                    let attempts = cpu_attempts.lock().unwrap();
                                    attempts.iter().sum::<u64>()
                                };
                                let total = gpu_total + cpu_total;

                                let elapsed = start_time.elapsed().as_secs_f64();
                                if elapsed > 0.0 {
                                    let rate = total as f64 / elapsed;
                                    println!(
                                        "Progress: {} attempts (CUDA GPU: {}, CPU: {}), {} keys/sec",
                                        total.to_formatted_string(&Locale::en),
                                        gpu_total.to_formatted_string(&Locale::en),
                                        cpu_total.to_formatted_string(&Locale::en),
                                        (rate as u64).to_formatted_string(&Locale::en)
                                    );
                                }
                            }
                        }
                    }
                }

                // Final update of this CPU thread's attempts
                {
                    let mut attempts = cpu_attempts.lock().unwrap();
                    attempts[thread_id] = local_attempts;
                }
            })
        })
        .collect();

    // Wait for completion (either GPU or CPU finds a match)
    gpu_handle.join().unwrap();
    for handle in cpu_handles {
        handle.join().unwrap();
    }

    // Output results
    let result_data = result.lock().unwrap().take();
    if let Some((ext_id, key_data, _winning_attempts, winner)) = result_data {
        let duration = start_time.elapsed().as_secs_f64();

        // Calculate total attempts across GPU and all CPU threads
        let gpu_total = *gpu_attempts.lock().unwrap();
        let cpu_total = {
            let attempts = cpu_attempts.lock().unwrap();
            attempts.iter().sum::<u64>()
        };
        let total = gpu_total + cpu_total;

        let rate = total as f64 / duration;

        println!("\n🎉 Match found by {}!", winner);
        println!("Extension ID: {}", ext_id);
        println!(
            "Total attempts: {} (CUDA GPU: {}, CPU: {})",
            total.to_formatted_string(&Locale::en),
            gpu_total.to_formatted_string(&Locale::en),
            cpu_total.to_formatted_string(&Locale::en)
        );
        println!("Duration: {:.2} seconds", duration);
        println!(
            "Rate: {} keys/second",
            (rate as u64).to_formatted_string(&Locale::en)
        );

        // Save files
        save_key_files(&key_data);

        // Print base64 for manifest
        let base64_key = base64::engine::general_purpose::STANDARD.encode(&key_data);
        println!("\nPublic key for manifest.json:");
        println!("{}", base64_key);
    }
}

#[cfg(feature = "cuda")]
fn run_cuda_vanity_id_generator(desired_prefix: &str, batch_size: u64) {
    let start_time = Instant::now();
    let mut total_attempts = 0u64;
    let mut last_progress_time = Instant::now();

    // Initialize CUDA GPU
    let gpu = match CudaVanityGenerator::new() {
        Ok(gpu) => {
            println!("CUDA GPU Device: {}", gpu.get_device_name());
            println!("Max threads per block: {}", gpu.get_max_threads_per_block());
            println!(
                "Batch size: {}",
                batch_size.to_formatted_string(&Locale::en)
            );
            gpu
        }
        Err(e) => {
            eprintln!("Failed to initialize CUDA GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Use independent counter ranges like CPU implementation
    // Each batch gets a unique range to avoid overlap with other potential GPU instances
    let mut batch_id = 0u64;

    loop {
        // Calculate starting counter for this batch using independent ranges
        let batch_start_counter = batch_id * batch_size;

        match gpu.search_vanity_id(desired_prefix, batch_start_counter, batch_size) {
            Ok(Some((found_counter, key_data))) => {
                // Found a match!
                total_attempts += found_counter - batch_start_counter + 1;

                let duration = start_time.elapsed().as_secs_f64();
                let rate = total_attempts as f64 / duration;

                // Generate extension ID for display
                let hash = Sha256::digest(&key_data);
                let extension_id = hash_to_extension_id(&hash);

                println!("\n🎉 Match found!");
                println!("Extension ID: {}", extension_id);
                println!(
                    "Total attempts: {}",
                    total_attempts.to_formatted_string(&Locale::en)
                );
                println!("Duration: {:.2} seconds", duration);
                println!(
                    "Rate: {} keys/second",
                    (rate as u64).to_formatted_string(&Locale::en)
                );

                // Save files
                save_key_files(&key_data);

                // Print base64 for manifest
                let base64_key = base64::engine::general_purpose::STANDARD.encode(&key_data);
                println!("\nPublic key for manifest.json:");
                println!("{}", base64_key);
                break;
            }
            Ok(None) => {
                // No match in this batch, continue with next batch
                total_attempts += batch_size;
                batch_id += 1;

                // Print progress every second
                let now = Instant::now();
                if now.duration_since(last_progress_time).as_secs() >= 1 {
                    last_progress_time = now;
                    let elapsed = start_time.elapsed().as_secs_f64();
                    if elapsed > 0.0 {
                        let rate = total_attempts as f64 / elapsed;
                        println!(
                            "Progress: {} attempts, {} keys/sec",
                            total_attempts.to_formatted_string(&Locale::en),
                            (rate as u64).to_formatted_string(&Locale::en)
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("CUDA GPU error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn save_key_files(key_data: &[u8]) {
    // Save DER format
    std::fs::write("public_key.der", key_data).expect("Failed to write DER file");
    println!("Saved: public_key.der");

    // Save PEM format
    let base64_key = base64::engine::general_purpose::STANDARD.encode(key_data);
    let pem_content = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
        base64_key
            .chars()
            .collect::<Vec<_>>()
            .chunks(64)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    );

    std::fs::write("public_key.pem", pem_content).expect("Failed to write PEM file");
    println!("Saved: public_key.pem");
}
