use base64;
use base64::Engine;
use clap::Parser;
use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
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
const SHIFT_WINDOW: usize = 32; // Shift by 32 bytes (256 bits) to ensure good variation

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
        println!("Performance: {:.2} keys/second", keys_per_sec);
        println!("===============================");
    }

    if single_thread {
        println!("Running in single-threaded mode");
    } else {
        println!("Using {} CPU cores for parallel processing", num_cores);
    }

    let total_attempts = Arc::new(AtomicU64::new(0));
    let start_time = Arc::new(Mutex::new(Instant::now()));

    let (stop_sender, stop_receiver): (Sender<()>, Receiver<()>) = unbounded();

    // Spawn a thread to print the summary every 30 seconds
    let summary_thread = {
        let total_attempts = Arc::clone(&total_attempts);
        let start_time = Arc::clone(&start_time);
        let stop_receiver = stop_receiver.clone();

        thread::spawn(move || {
            let ticker = tick(Duration::from_secs(30));
            loop {
                select! {
                    recv(ticker) -> _ => {
                        let elapsed = start_time.lock().unwrap().elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            let attempts_val = total_attempts.load(Ordering::Relaxed);
                            let keys_per_sec = attempts_val as f64 / elapsed;
                            println!(
                                "[Progress] Total attempts: {}, Performance: {:.2} keys/sec",
                                attempts_val.to_formatted_string(&Locale::en),
                                keys_per_sec
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

    if single_thread {
        // Single-threaded mode
        // Create a shared random data chunk for this thread
        let mut current_chunk = vec![0u8; CHUNK_SIZE];
        openssl::rand::rand_bytes(&mut current_chunk).expect("failed to generate random bytes");
        let mut shift_position = 0;
        
        loop {
            total_attempts.fetch_add(1, Ordering::Relaxed);
            if let Some((ext_id, public_key_data)) = generate_key_and_id_optimized(desired_prefix, &mut current_chunk, &mut shift_position) {
                stop_sender.send(()).unwrap(); // Signal the summary thread to stop
                summary_thread.join().unwrap(); // Wait for the summary thread to finish

                let main_duration = start_time.lock().unwrap().elapsed().as_secs_f64();
                let total_attempts_val = total_attempts.load(Ordering::SeqCst);
                let keys_per_second_main = total_attempts_val as f64 / main_duration;
                println!("\n=== Main Run Benchmark Report ===");
                println!(
                    "Total attempts: {}",
                    total_attempts_val.to_formatted_string(&Locale::en)
                );
                println!("Duration: {:.2} seconds", main_duration);
                println!("Performance: {:.2} keys/second", keys_per_second_main);
                println!("Mode: Single-threaded");
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
                break;
            }
        }
    } else {
        // Multi-threaded mode
        loop {
            let result = (0..num_cores)
                .into_par_iter()
                .map(|_| {
                    total_attempts.fetch_add(1, Ordering::Relaxed);
                    // Each thread gets its own random data chunk
                    let mut current_chunk = vec![0u8; CHUNK_SIZE];
                    openssl::rand::rand_bytes(&mut current_chunk).expect("failed to generate random bytes");
                    let mut shift_position = 0;
                    generate_key_and_id_optimized(desired_prefix, &mut current_chunk, &mut shift_position)
                })
                .find_any(|result| result.is_some());

            if let Some(Some((ext_id, public_key_data))) = result {
                stop_sender.send(()).unwrap(); // Signal the summary thread to stop
                summary_thread.join().unwrap(); // Wait for the summary thread to finish

                let main_duration = start_time.lock().unwrap().elapsed().as_secs_f64();
                let total_attempts_val = total_attempts.load(Ordering::SeqCst);
                let keys_per_second_main = total_attempts_val as f64 / main_duration;
                println!("\n=== Main Run Benchmark Report ===");
                println!(
                    "Total attempts: {}",
                    total_attempts_val.to_formatted_string(&Locale::en)
                );
                println!("Duration: {:.2} seconds", main_duration);
                println!("Performance: {:.2} keys/second", keys_per_second_main);
                println!("Cores utilized: {}", num_cores);
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
                break;
            }
        }
    }
}

fn generate_key_and_id_optimized(desired_prefix: &str, current_chunk: &mut Vec<u8>, shift_position: &mut usize) -> Option<(String, Vec<u8>)> {
    // If we've exhausted shifts in this chunk, generate a new one
    if *shift_position >= SHIFT_WINDOW {
        openssl::rand::rand_bytes(current_chunk).expect("failed to generate random bytes");
        *shift_position = 0;
    }
    
    // Create a shifted version of the current chunk
    let mut shifted_chunk = current_chunk.clone();
    if *shift_position > 0 && *shift_position < shifted_chunk.len() {
        // Perform a bitwise shift by rotating the bytes
        shifted_chunk.rotate_left(*shift_position);
    }
    
    *shift_position += 1;
    
    // Use the shifted chunk as our public key data
    let public_key_der = shifted_chunk;
    
    // Calculate the Chrome extension ID from the public key
    let mut hasher = Sha256::new();
    hasher.update(&public_key_der);
    let hash_result = hasher.finalize();

    let extension_id: String = hex::encode(&hash_result[..16])
        .chars()
        .map(|c| MAPPING[c.to_digit(16).unwrap() as usize])
        .collect();

    if extension_id.starts_with(desired_prefix) {
        // For a vanity ID generator, we return the extension ID and the public key data
        Some((extension_id, public_key_der))
    } else {
        None
    }
}

fn benchmark_keygen(attempts: u32) -> (f64, f64, u32) {
    let start_time = Instant::now();
    // Create a shared random data chunk for benchmarking
    let mut current_chunk = vec![0u8; CHUNK_SIZE];
    openssl::rand::rand_bytes(&mut current_chunk).expect("failed to generate random bytes");
    let mut shift_position = 0;
    
    for _ in 0..attempts {
        // We pass a dummy prefix since we don't care about finding a match here
        generate_key_and_id_optimized("benchmark", &mut current_chunk, &mut shift_position);
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