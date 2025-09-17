use base64;
use base64::Engine;
use clap::Parser;
use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use num_format::{Locale, ToFormattedString};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ok")]
    prefix: String,

    #[arg(short, long, default_value_t = num_cpus::get())]
    cores: usize,

    #[arg(long)]
    benchmark: bool,

    #[arg(long)]
    single_thread: bool,
}

const MAPPING: [char; 16] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
];

// Constants for the bit-shifting optimization
const CHUNK_SIZE: usize = 256; // Generate 256 bytes of random data at a time
const SHIFT_WINDOW: usize = 256; // Shift by 1 byte at a time for 256 shifts

fn main() {
    let cli = Cli::parse();

    let desired_prefix = &cli.prefix;
    let num_cores = cli.cores;
    let single_thread = cli.single_thread;

    if cli.benchmark {
        println!("=== Running Single Core Benchmark (100 attempts) ===");
        let (keys_per_sec, duration, attempts) = benchmark_keygen(100);
        println!(
            "Single Core Benchmark: Generated {} random public key data with ID computation",
            attempts.to_formatted_string(&Locale::en)
        );
        println!("Duration: {:.2} seconds", duration);
        println!("Performance: {} keys/second", (keys_per_sec as u64).to_formatted_string(&Locale::en));
        println!("===============================");
    }

    let thread_count = if single_thread { 1 } else { num_cores };
    if thread_count == 1 {
        println!("Running in single-threaded mode");
    } else {
        println!("Using {} CPU cores for parallel processing", thread_count);
    }
    
    run_vanity_id_generator(desired_prefix, thread_count);
}

fn run_vanity_id_generator(desired_prefix: &str, num_threads: usize) {
    let start_time = Instant::now();
    let total_attempts = Arc::new(AtomicU64::new(0));
    let found = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));
    
    // Channel for stopping the summary thread
    let (stop_sender, stop_receiver): (Sender<()>, Receiver<()>) = unbounded();
    
    // Spawn a thread to print the summary every 30 seconds
    let summary_thread = {
        let total_attempts = Arc::clone(&total_attempts);
        let start_time = start_time.clone();
        let stop_receiver = stop_receiver.clone();
        
        thread::spawn(move || {
            let ticker = tick(Duration::from_secs(30));
            loop {
                select! {
                    recv(ticker) -> _ => {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            let attempts_val = total_attempts.load(Ordering::Relaxed);
                            let keys_per_sec = attempts_val as f64 / elapsed;
                            println!(
                                "[Progress] Total attempts: {}, Performance: {} keys/sec",
                                attempts_val.to_formatted_string(&Locale::en),
                                (keys_per_sec as u64).to_formatted_string(&Locale::en)
                            );
                        }
                    },
                    recv(stop_receiver) -> _ => {
                        break;
                    }
                }
            }
        })
    };
    
    let mut handles = vec![];
    let desired_prefix = desired_prefix.to_string();
    
    for _ in 0..num_threads {
        let desired_prefix = desired_prefix.clone();
        let total_attempts = Arc::clone(&total_attempts);
        let found = Arc::clone(&found);
        let result = Arc::clone(&result);
        
        let handle = thread::spawn(move || {
            // Double buffer to avoid cloning in the hot loop
            let mut double_chunk = vec![0u8; CHUNK_SIZE * 2];
            openssl::rand::rand_bytes(&mut double_chunk[0..CHUNK_SIZE]).expect("failed to generate random bytes");
            double_chunk.copy_within(0..CHUNK_SIZE, CHUNK_SIZE);
            
            let mut shift_position = 0;
            
            while !found.load(Ordering::Relaxed) {
                total_attempts.fetch_add(1, Ordering::Relaxed);
                
                if let Some((ext_id, public_key_data)) = generate_key_and_id_optimized(&desired_prefix, &mut double_chunk, &mut shift_position) {
                    if !found.swap(true, Ordering::Relaxed) {
                        let mut result_lock = result.lock().unwrap();
                        *result_lock = Some((ext_id, public_key_data));
                    }
                    break;
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for any thread to find a match
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Signal the summary thread to stop and wait for it to finish
    stop_sender.send(()).unwrap();
    summary_thread.join().unwrap();
    
    // Get the result
    let result_val = result.lock().unwrap().take();
    if let Some((ext_id, public_key_data)) = result_val {
        let main_duration = start_time.elapsed().as_secs_f64();
        let total_attempts_val = total_attempts.load(Ordering::SeqCst);
        let keys_per_second_main = total_attempts_val as f64 / main_duration;
        println!("\n=== Main Run Benchmark Report ===");
        println!(
            "Total attempts: {}",
            total_attempts_val.to_formatted_string(&Locale::en)
        );
        if num_threads == 1 {
            println!("Duration: {:.2} seconds", main_duration);
            println!("Performance: {} keys/second", (keys_per_second_main as u64).to_formatted_string(&Locale::en));
            println!("Mode: Single-threaded");
        } else {
            println!("Duration: {:.2} seconds", main_duration);
            println!("Performance: {} keys/second", (keys_per_second_main as u64).to_formatted_string(&Locale::en));
            println!("Cores utilized: {}", num_threads);
        }
        println!("===============================");

        println!("\nMatch found!");
        println!("Extension ID: {}", ext_id);
        // Save the public key in DER format
        std::fs::write("public_key.der", &public_key_data).expect("Unable to write public key to file");
        println!("Generated public key data saved to 'public_key.der'");
        
        // Save the public key in PEM format
        save_public_key_as_pem(&public_key_data, "public_key.pem");
        
        // Print the base64-encoded public key for easy copying
        let base64_key = base64::engine::general_purpose::STANDARD.encode(&public_key_data);
        println!("\nPublic key (base64-encoded, for Chrome extension manifest):");
        println!("{}", base64_key);
        println!("\nThis key can be directly copied into your Chrome extension's manifest.json file.");
    }
}

fn generate_key_and_id_optimized(desired_prefix: &str, double_chunk: &mut [u8], shift_position: &mut usize) -> Option<(String, Vec<u8>)> {
    // If we've exhausted shifts, generate a new random chunk
    if *shift_position >= SHIFT_WINDOW {
        openssl::rand::rand_bytes(&mut double_chunk[0..CHUNK_SIZE]).expect("failed to generate random bytes");
        double_chunk.copy_within(0..CHUNK_SIZE, CHUNK_SIZE);
        *shift_position = 0;
    }

    // Get a slice representing the shifted chunk, no allocation
    let shifted_slice = &double_chunk[*shift_position..*shift_position + CHUNK_SIZE];
    *shift_position += 1;

    // Calculate the Chrome extension ID from the public key
    let mut hasher = Sha256::new();
    hasher.update(shifted_slice);
    let hash_result = hasher.finalize();

    let extension_id: String = hex::encode(&hash_result[..16])
        .chars()
        .map(|c| MAPPING[c.to_digit(16).unwrap() as usize])
        .collect();

    if extension_id.starts_with(desired_prefix) {
        // Only allocate and return the Vec when a match is found
        Some((extension_id, shifted_slice.to_vec()))
    } else {
        None
    }
}

fn benchmark_keygen(attempts: u32) -> (f64, f64, u32) {
    let start_time = Instant::now();
    // Create a shared random data chunk for benchmarking
    let mut double_chunk = vec![0u8; CHUNK_SIZE * 2];
    openssl::rand::rand_bytes(&mut double_chunk[0..CHUNK_SIZE]).expect("failed to generate random bytes");
    double_chunk.copy_within(0..CHUNK_SIZE, CHUNK_SIZE);
    let mut shift_position = 0;
    
    for _ in 0..attempts {
        // We pass a dummy prefix since we don't care about finding a match here
        generate_key_and_id_optimized("benchmark", &mut double_chunk, &mut shift_position);
    }
    let duration = start_time.elapsed().as_secs_f64();
    let keys_per_second = attempts as f64 / duration;
    (keys_per_second, duration, attempts)
}

/// Save the public key in PEM format
fn save_public_key_as_pem(public_key_data: &[u8], filename: &str) {
    // Create PEM header and footer
    let pem_header = "-----BEGIN PUBLIC KEY-----\n";
    let pem_footer = "\n-----END PUBLIC KEY-----\n";
    
    // Base64 encode the public key data with 64-character lines
    let base64_key = base64::engine::general_purpose::STANDARD.encode(public_key_data);
    let formatted_base64: String = base64_key
        .chars()
        .collect::<Vec<char>>()
        .chunks(64)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join("\n");
    
    // Combine all parts
    let pem_content = format!("{}{}{}", pem_header, formatted_base64, pem_footer);
    
    // Write to file
    std::fs::write(filename, pem_content).expect("Unable to write public key PEM file");
    println!("Generated public key saved to '{}'", filename);
}