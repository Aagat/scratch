use base64;
use base64::Engine;
use clap::Parser;
use num_format::{Locale, ToFormattedString};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ok")]
    prefix: String,

    #[arg(short, long, default_value_t = num_cpus::get())]
    cores: usize,

    #[arg(long)]
    single_thread: bool,
}

const MAPPING: [char; 16] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
];

fn main() {
    let cli = Cli::parse();
    let thread_count = if cli.single_thread { 1 } else { cli.cores };

    println!("Searching for extension ID with prefix: {}", cli.prefix);
    println!("Using {} thread(s)", thread_count);

    run_vanity_id_generator(&cli.prefix, thread_count);
}

fn run_vanity_id_generator(desired_prefix: &str, num_threads: usize) {
    let start_time = Instant::now();
    let total_attempts = Arc::new(AtomicU64::new(0));
    let found = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));

    // Spawn worker threads
    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let prefix = desired_prefix.to_string();
            let attempts = Arc::clone(&total_attempts);
            let found = Arc::clone(&found);
            let result = Arc::clone(&result);
            let start_time = start_time.clone();

            thread::spawn(move || {
                while !found.load(Ordering::Relaxed) {
                    let counter = attempts.fetch_add(1, Ordering::Relaxed);

                    if let Some((ext_id, key_data)) = try_generate_match(&prefix, counter) {
                        if !found.swap(true, Ordering::Relaxed) {
                            *result.lock().unwrap() = Some((ext_id, key_data));
                        }
                        break;
                    }

                    // Only thread 0 prints progress every 10M attempts
                    if thread_id == 0 && counter % 10_000_000 == 0 {
                        let total = attempts.load(Ordering::Relaxed);
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
            })
        })
        .collect();

    // Wait for completion
    for handle in handles {
        handle.join().unwrap();
    }

    // Output results
    let result_data = result.lock().unwrap().take();
    if let Some((ext_id, key_data)) = result_data {
        let duration = start_time.elapsed().as_secs_f64();
        let total = total_attempts.load(Ordering::SeqCst);
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

fn try_generate_match(desired_prefix: &str, counter: u64) -> Option<(String, [u8; 32])> {
    // Generate key data from counter
    let key_data = generate_key_data(counter);

    // Hash the key data
    let hash = Sha256::digest(&key_data);

    // Check if hash matches desired prefix
    if hash_matches_prefix(&hash, desired_prefix) {
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

fn hash_matches_prefix(hash: &[u8], prefix: &str) -> bool {
    let prefix_bytes = prefix.as_bytes();

    for (i, &expected_char) in prefix_bytes.iter().enumerate() {
        let hash_byte = hash[i / 2];
        let nibble = if i % 2 == 0 {
            hash_byte >> 4
        } else {
            hash_byte & 0x0F
        };
        let actual_char = MAPPING[nibble as usize];

        if actual_char as u8 != expected_char {
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
