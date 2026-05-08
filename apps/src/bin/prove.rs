use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

use alloy_dyn_abi::TypedData;
use alloy_primitives::{Address, Bytes, Signature};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolValue;
use anyhow::{bail, Result};
use clap::Parser;
use guests::SINGLE_SIGN_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use url::Url;

use common::{find_concatenated_json_ranges, Input, Output};

mod contracts {
    alloy_sol_types::sol!(
        #![sol(rpc, all_derives)]
        interface SingleSign {
            function isValidSignature(bytes32 hash, bytes calldata signature) external view returns (bytes4);
        }
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    match dotenvy::dotenv() {
        Ok(path) => debug!("Loaded environment variables from {:?}", path),
        Err(e) if e.not_found() => debug!("No .env file found"),
        Err(e) => bail!("failed to load .env file: {}", e),
    }

    let args = Args::parse();

    let signer = PrivateKeySigner::from_bytes(&args.private_key.to_bytes()).unwrap();
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(args.rpc_url.clone());
    let account_address = args.account_address;

    let file_bytes = fs::read(&args.file_path)?;
    let typed_data_concat: Bytes = Bytes::from(file_bytes);
    let signature: Signature = args.signature;
    let signer: Address = args.signer;

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

        info!("Proving input #{i}");
        let prove_info = tokio::task::spawn_blocking(move || {
            let env = ExecutorEnv::builder()
                .write(&input)
                .unwrap()
                .build()
                .unwrap();
            let prover = default_prover();
            prover.prove_with_ctx(
                env,
                &VerifierContext::default(),
                SINGLE_SIGN_ELF,
                &ProverOpts::groth16(),
            )
        })
        .await
        .expect("proving task failed")
        .unwrap();

        let receipt = prove_info.receipt;

        let output: Output = Output::abi_decode(&receipt.journal.bytes).unwrap();
        info!("Guest output #{i} -> {:?}", output);

        receipt.verify(guests::SINGLE_SIGN_ID).unwrap();

        let typed_data: TypedData = serde_json::from_str(
            &String::from_utf8(typed_data_concat[range.start..range.end].to_vec()).unwrap(),
        )
        .unwrap();
        let digest = typed_data.eip712_signing_hash().unwrap();
        assert_eq!(digest, output.digest);

        let seal = receipt.inner.groth16()?.seal.clone();

        let single_sign = contracts::SingleSign::new(account_address.clone(), provider.clone());
        let is_valid = single_sign
            .isValidSignature(digest, seal.into())
            .call()
            .await
            .unwrap();
        info!("Is valid: {:?}", is_valid);
    }

    Ok(())
}
