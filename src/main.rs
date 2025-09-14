
use clap::Parser;
use num_format::{Locale, ToFormattedString};
use pkcs8::EncodePrivateKey;
use rayon::prelude::*;
use rsa::{pkcs1::EncodeRsaPublicKey, RsaPrivateKey};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ok")]
    prefix: String,

    #[arg(short, long, default_value_t = num_cpus::get())]
    cores: usize,

    #[arg(long)]
    benchmark: bool,
}

const MAPPING: [char; 16] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
];

fn main() {
    let cli = Cli::parse();

    let desired_prefix = &cli.prefix;
    let num_cores = cli.cores;

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

    println!("Using {} CPU cores for parallel processing", num_cores);

    let total_attempts = AtomicU64::new(0);
    let start_time = Instant::now();

    loop {
        let result = (0..num_cores)
            .into_par_iter()
            .map(|_| {
                total_attempts.fetch_add(1, Ordering::SeqCst);
                generate_key_and_id(desired_prefix)
            })
            .find_any(|result| result.is_some());

        if let Some(Some((ext_id, pem))) = result {
            let main_duration = start_time.elapsed().as_secs_f64();
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

fn generate_key_and_id(desired_prefix: &str) -> Option<(String, String)> {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");

    let public_key_der = private_key.to_public_key().to_pkcs1_der().unwrap();

    let mut hasher = Sha256::new();
    hasher.update(public_key_der.as_bytes());
    let hash_result = hasher.finalize();

    let hex_digest = hex::encode(&hash_result[..16]);

    let extension_id: String = hex_digest
        .chars()
        .map(|c| MAPPING[c.to_digit(16).unwrap() as usize])
        .collect();

    if extension_id.starts_with(desired_prefix) {
        let pem = private_key
            .to_pkcs8_pem(pkcs8::LineEnding::LF)
            .unwrap();
        Some((extension_id, pem.to_string()))
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
