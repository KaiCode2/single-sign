use std::fs;
use std::path::PathBuf;

use alloy_dyn_abi::TypedData;
use alloy_primitives::{hex, Address, Bytes, B256};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolValue;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use common::{find_concatenated_json_ranges, Input, Output};
use guests::{SINGLE_SIGN_ELF, SINGLE_SIGN_ID};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{default_prover, sha::Digest, ExecutorEnv, ProverOpts, VerifierContext};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tracing::info;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// File containing the concatenated compact EIP-712 typed-data JSON.
    #[clap(long, value_name = "FILE", default_value = "examples/typed_data_concat.json")]
    input_file: PathBuf,

    /// Where to write the fixture JSON.
    #[clap(long, value_name = "FILE", default_value = "artifacts/single_sign_fixture.json")]
    out_path: PathBuf,

    /// Index of the message within the concatenation to prove (0-based).
    #[clap(long, default_value_t = 0)]
    target_index: usize,

    /// Private key for the signing EOA. Falls back to USER_PRIVATE_KEY env, then a random key.
    #[clap(long, env = "USER_PRIVATE_KEY")]
    private_key: Option<PrivateKeySigner>,
}

#[derive(Serialize)]
struct Fixture {
    #[serde(rename = "imageId")]
    image_id: String,
    signer: String,
    hash: String,
    journal: String,
    #[serde(rename = "journalDigest")]
    journal_digest: String,
    seal: String,
}

fn hex0x(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let _ = dotenvy::dotenv();

    let args = Args::parse();

    let signer = args.private_key.unwrap_or_else(PrivateKeySigner::random);
    let signer_address: Address = signer.address();

    let typed_data_concat: Bytes = Bytes::from(fs::read(&args.input_file)?);
    let concat_str = std::str::from_utf8(&typed_data_concat)
        .context("input file is not valid UTF-8")?;

    let ranges = find_concatenated_json_ranges(concat_str)?;
    let range = ranges
        .get(args.target_index)
        .ok_or_else(|| anyhow!("target_index {} out of range (have {} messages)", args.target_index, ranges.len()))?;

    let signature = signer.sign_message_sync(&typed_data_concat)?;

    let typed: TypedData = serde_json::from_str(&concat_str[range.start..range.end])
        .context("slice is not valid EIP-712 typed-data JSON")?;
    let eip712_digest: B256 = typed.eip712_signing_hash()?;

    let input = Input {
        signer: signer_address,
        signature,
        typed_data_concat: typed_data_concat.clone(),
        digest_range: range.clone(),
    };

    info!("Proving message #{} (range {:?})", args.target_index, range);
    let env = ExecutorEnv::builder().write(&input)?.build()?;
    let receipt = default_prover()
        .prove_with_ctx(env, &VerifierContext::default(), SINGLE_SIGN_ELF, &ProverOpts::groth16())?
        .receipt;

    receipt
        .verify(SINGLE_SIGN_ID)
        .context("locally generated receipt failed verification")?;

    let output = Output::abi_decode(&receipt.journal.bytes)
        .context("guest journal is not abi-encoded Output")?;
    if output.signer != signer_address {
        return Err(anyhow!(
            "guest signer {:#x} != fixture signer {:#x}",
            output.signer,
            signer_address
        ));
    }
    if output.digest != eip712_digest {
        return Err(anyhow!("guest digest does not match recomputed EIP-712 digest"));
    }

    let journal_bytes = receipt.journal.bytes.clone();
    let journal_digest = Sha256::digest(&journal_bytes);
    let seal = encode_seal(&receipt)?;

    let image_id = Digest::from(SINGLE_SIGN_ID);

    let fixture = Fixture {
        image_id: hex0x(image_id.as_bytes()),
        signer: format!("{:#x}", signer_address),
        hash: hex0x(eip712_digest.as_slice()),
        journal: hex0x(&journal_bytes),
        journal_digest: hex0x(journal_digest.as_slice()),
        seal: hex0x(&seal),
    };

    if let Some(parent) = args.out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.out_path, serde_json::to_vec_pretty(&fixture)?)?;
    info!("Fixture written to {}", args.out_path.display());

    Ok(())
}
