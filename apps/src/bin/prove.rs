use app::PERMIT2_ADDRESS;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

use alloy_dyn_abi::TypedData;
use alloy_primitives::{Address, Bytes, Signature, U256};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use anyhow::{bail, Result};
use clap::Parser;
use guests::SINGLE_SIGN_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use url::Url;

use common::{find_concatenated_json_ranges, Input, Output};

mod contracts {
    alloy_sol_types::sol!(
        #![sol(rpc, all_derives)]
        Permit2,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../contracts/out/IPermit2.sol/IPermit2.json"
        )
    );
}

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

#[tokio::main]
async fn main() -> Result<()> {
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
    let rpc_url = &args.rpc_url;
    let private_key = &args.private_key;
    let account_address = &args.account_address;
    let _program_url = &args.program_url;

    let signer = PrivateKeySigner::from_bytes(&private_key.to_bytes()).unwrap();
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(rpc_url.clone());

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
        let prove_info = prover
            .prove_with_ctx(
                env,
                &VerifierContext::default(),
                SINGLE_SIGN_ELF,
                &ProverOpts::groth16(),
            )
            .unwrap();
        let receipt = prove_info.receipt;

        // Decode public output committed by the guest
        let output: Output = receipt.journal.decode().unwrap();
        info!(
            "Guest output #{i} -> signer: {:#x}, digest: 0x{}",
            output.signer,
            alloy_primitives::hex::encode(output.digest),
        );

        // Optional verification example (requires SINGLE_SIGN_ID):
        receipt.verify(guests::SINGLE_SIGN_ID).unwrap();

        let typed_data: TypedData = serde_json::from_str(
            &String::from_utf8(typed_data_concat[range.start..range.end].to_vec()).unwrap(),
        )
        .unwrap();
        let digest = typed_data.eip712_signing_hash().unwrap();
        assert_eq!(digest, output.digest);

        if typed_data.primary_type == "PermitTransferFrom" {
            // Try calling PermitTransferFrom using Permit2
            let seal = receipt.inner.groth16()?.seal.clone();
            let permit = contracts::ISignatureTransfer::PermitTransferFrom {
                permitted: contracts::ISignatureTransfer::TokenPermissions {
                    token: typed_data.message["permitted"]["token"]
                        .as_str()
                        .unwrap()
                        .parse()
                        .unwrap(),
                    amount: U256::from(typed_data.message["permitted"]["amount"].as_u64().unwrap()),
                },
                nonce: U256::from(typed_data.message["nonce"].as_u64().unwrap()),
                deadline: U256::from(typed_data.message["deadline"].as_u64().unwrap()),
            };
            let permit2 = contracts::Permit2::new(PERMIT2_ADDRESS, provider.clone());
            let tx = permit2
                .permitTransferFrom_0(
                    permit,
                    contracts::ISignatureTransfer::SignatureTransferDetails {
                        to: signer.clone(),
                        requestedAmount: U256::from(1_000_000_000_000_000_000u128),
                    },
                    account_address.clone(),
                    Bytes::from(seal),
                )
                .send()
                .await
                .unwrap();
            let receipt = tx.get_receipt().await.unwrap();
            info!("Transaction receipt: {:?}", receipt);
        }
    }

    Ok(())
}
