
use rsa::{
    pkcs1::EncodeRsaPublicKey,
    RsaPrivateKey,
};
use sha2::{Digest, Sha256};
use std::time::Instant;
use pkcs8::EncodePrivateKey;
use rayon::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "ok")]
    prefix: String,

    #[arg(short, long, default_value_t = num_cpus::get())]
    cores: usize,
}

const MAPPING: [char; 16] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
];

fn main() {
    let cli = Cli::parse();

    let desired_prefix = &cli.prefix;
    let num_cores = cli.cores;
    println!("Using {} CPU cores for parallel processing", num_cores);

    let start_time = Instant::now();

    loop {
        let result = (0..num_cores).into_par_iter()
            .map(|_| generate_key_and_id(desired_prefix))
            .find_any(|result| result.is_some());

        if let Some(Some((ext_id, pem))) = result {
            println!("Match found!");
            println!("Extension ID: {}", ext_id);
            std::fs::write("key.pem", pem).expect("Unable to write key to file");
            println!("Private key saved to 'key.pem'. Keep it secure!");
            break;
        }
    }

    let duration = start_time.elapsed();
    println!("Duration: {:.2?} seconds", duration);
}

fn generate_key_and_id(desired_prefix: &str) -> Option<(String, String)> {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");

    let public_key_der = private_key
        .to_public_key()
        .to_pkcs1_der()
        .unwrap();

    let mut hasher = Sha256::new();
    hasher.update(public_key_der.as_bytes());
    let hash_result = hasher.finalize();

    let hex_digest = hex::encode(&hash_result[..16]);

    let extension_id: String = hex_digest
        .chars()
        .map(|c| MAPPING[c.to_digit(16).unwrap() as usize])
        .collect();

    if extension_id.starts_with(desired_prefix) {
        let pem = private_key.to_pkcs8_pem(pkcs8::LineEnding::LF).unwrap();
        Some((extension_id, pem.to_string()))
    } else {
        None
    }
}
