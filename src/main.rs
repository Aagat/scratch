use clap::Parser;
use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
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

fn main() {
    let cli = Cli::parse();

    let desired_prefix = &cli.prefix;
    let num_cores = cli.cores;
    let single_thread = cli.single_thread;

    if cli.benchmark {
        println!("=== Running Single Core Benchmark (100 attempts) ===");
        let (keys_per_sec, duration, attempts) = benchmark_keygen(100);
        println!(
            "Single Core Benchmark: Generated {} RSA 2048-bit keys with ID computation",
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
        loop {
            total_attempts.fetch_add(1, Ordering::Relaxed);
            if let Some((ext_id, pem)) = generate_key_and_id(desired_prefix) {
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
                std::fs::write("key.pem", pem).expect("Unable to write key to file");
                println!("Private key saved to 'key.pem'. Keep it secure!");
                break;
            }
        }
    } else {
        // Multi-threaded mode (existing code)
        loop {
            let result = (0..num_cores)
                .into_par_iter()
                .map(|_| {
                    total_attempts.fetch_add(1, Ordering::Relaxed);
                    generate_key_and_id(desired_prefix)
                })
                .find_any(|result| result.is_some());

            if let Some(Some((ext_id, pem))) = result {
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
                std::fs::write("key.pem", pem).expect("Unable to write key to file");
                println!("Private key saved to 'key.pem'. Keep it secure!");
                break;
            }
        }
    }
}

fn generate_key_and_id(desired_prefix: &str) -> Option<(String, String)> {
    let rsa = Rsa::generate(2048).expect("failed to generate a key");
    let pkey = PKey::from_rsa(rsa).expect("failed to create PKey");

    let public_key_der = pkey.public_key_to_der().expect("failed to DER encode public key");

    let mut hasher = Sha256::new();
    hasher.update(&public_key_der);
    let hash_result = hasher.finalize();

    let extension_id: String = hex::encode(&hash_result[..16])
        .chars()
        .map(|c| MAPPING[c.to_digit(16).unwrap() as usize])
        .collect();

    if extension_id.starts_with(desired_prefix) {
        let pem = pkey.private_key_to_pem_pkcs8().expect("failed to PEM encode");
        Some((extension_id, String::from_utf8_lossy(&pem).to_string()))
    } else {
        None
    }
}

fn benchmark_keygen(attempts: u32) -> (f64, f64, u32) {
    let start_time = Instant::now();
    for _ in 0..attempts {
        // We pass a dummy prefix since we don't care about finding a match here
        generate_key_and_id("benchmark");
    }
    let duration = start_time.elapsed().as_secs_f64();
    let keys_per_second = attempts as f64 / duration;
    (keys_per_second, duration, attempts)
}