use std::fs;
use std::path::PathBuf;

use alloy_primitives::{keccak256, hex, Address, Bytes, Signature, B256};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;
use clap::Parser;

/// CLI to sign over a file's raw bytes and print digest, signature, and signer.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the input file whose bytes will be signed.
    #[clap(long, value_name = "FILE")]
    file_path: PathBuf,

    /// Optional private key to use for signing; if omitted, a random key is generated.
    #[clap(long, env = "USER_PRIVATE_KEY")]
    private_key: Option<PrivateKeySigner>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    // Read file bytes
    let file_bytes = fs::read(&args.file_path)?;
    let file_bytes = Bytes::from(file_bytes);

    // Compute keccak256 digest of file bytes
    let digest: B256 = keccak256(&file_bytes);

    // Obtain signer (existing or random)
    let signer = match args.private_key {
        Some(pk) => pk,
        None => PrivateKeySigner::random(),
    };
    let signer_address: Address = signer.address();

    // Sign raw bytes using EIP-191 personal message mode via sign_message_sync
    let signature: Signature = signer.sign_message_sync(&file_bytes)?;

    println!("File: {}", args.file_path.display());
    println!("Digest (keccak256): 0x{}", hex::encode(digest));
    println!("Signature: 0x{}", hex::encode(&signature.as_bytes()));
    println!("Signer: {:#x}", signer_address);

    Ok(())
}


