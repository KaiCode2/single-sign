use std::fs;
use std::path::PathBuf;
use tracing::{info, debug};

use alloy_primitives::{Address, Bytes, Signature};
use alloy_signer_local::PrivateKeySigner;
use anyhow::{bail, Result};
use clap::Parser;
use guests::SINGLE_SIGN_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv};
use url::Url;

use common::{find_concatenated_json_ranges, Input, Output};

/// CLI arguments for proving signatures over aggregated typed-data JSON.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the aggregated compact JSON string (concatenated typed-data JSONs).
    #[clap(long, value_name = "FILE")]
    file_path: PathBuf,

    /// Signer address that produced the provided signature.
    #[clap(long)]
    signer: Address,

    /// Signature over the raw `json_compact_all` bytes (65-byte hex string).
    #[clap(long)]
    signature: Signature,

    /// URL of the Ethereum RPC endpoint (retained for future use).
    #[clap(short, long, env)]
    rpc_url: Url,

    /// Private key used for future interactions.
    #[clap(long, env)]
    private_key: PrivateKeySigner,

    /// Address of a target contract.
    #[clap(short = 'a', long, env = "ACCOUNT_ADDRESS")]
    account_address: Address,

    /// URL where provers can download the program.
    #[clap(long, env)]
    program_url: Option<Url>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    // Load environment variables if present
    match dotenvy::dotenv() {
        Ok(path) => debug!("Loaded environment variables from {:?}", path),
        Err(e) if e.not_found() => debug!("No .env file found"),
        Err(e) => bail!("failed to load .env file: {}", e),
    }

    let args = Args::parse();

    // Retain RPC-related params for parity (not used in this local proving flow)
    let _rpc_url = &args.rpc_url;
    let _private_key = &args.private_key;
    let _account_address = &args.account_address;
    let _program_url = &args.program_url;

    // Read the aggregated compact JSON bytes from file
    let file_bytes = fs::read(&args.file_path)?;
    let typed_data_concat: Bytes = Bytes::from(file_bytes);
    let signature: Signature = args.signature;
    let signer: Address = args.signer;

    // Mock digest ranges (replace with a real parser implementation later)
    let digest_ranges =
        find_concatenated_json_ranges(&String::from_utf8(typed_data_concat.to_vec()).unwrap())?;
    info!("Digest ranges: {:?}", digest_ranges);

    for (i, range) in digest_ranges.iter().enumerate() {
        let input = Input {
            signer,
            signature,
            typed_data_concat: typed_data_concat.clone(),
            digest_range: range.clone(),
        };
        debug!("Input #{i}: {:?}", input);

        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();

        let prover = default_prover();
        info!("Proving input #{i}");
        let prove_info = prover.prove(env, SINGLE_SIGN_ELF).unwrap();
        let receipt = prove_info.receipt;

        // Decode public output committed by the guest
        let output: Output = receipt.journal.decode().unwrap();
        info!(
            "Guest output #{i} -> signer: {:#x}, digest: 0x{}",
            output.signer,
            alloy_primitives::hex::encode(output.digest),
        );

        // Optional verification example (requires SINGLE_SIGN_ID):
        // receipt.verify(SINGLE_SIGN_ID).unwrap();
    }

    Ok(())
}
