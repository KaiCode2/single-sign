use alloy_primitives::{keccak256, Address, Bytes, Signature, B256};
use anyhow::{anyhow, Result};

/// Verify an Ethereum ECDSA signature against an expected signer address.
/// - `message`: the message bytes (either already hashed or raw bytes)
/// - `signature`: 65-byte r||s||v signature (v = 27/28 or 0/1 or 35+ chain-ids are ok)
/// - `expected`: the address you expect as the signer
/// - `mode`: how to interpret `message`
///     - Raw32: `message` is a 32-byte prehash (use as-is)
///     - Keccak: `message` is arbitrary bytes; hash with keccak256(message)
///     - Personal: EIP-191; hash with keccak256("\x19Ethereum Signed Message:\n{len}" || message)
pub enum MessageMode {
    Raw32,
    Keccak,
    Personal,
}

pub fn verify_signature(
    message: Bytes,
    signature: Signature,
    expected: Address,
    mode: MessageMode,
) -> Result<bool> {
    // 1) Build the pre-hash we’ll recover from.
    let prehash: B256 = match mode {
        MessageMode::Raw32 => {
            if message.len() != 32 {
                return Err(anyhow!("Raw32 mode requires a 32-byte prehash"));
            }
            B256::from_slice(&message)
        }
        MessageMode::Keccak => keccak256(&message),
        MessageMode::Personal => {
            // EIP-191: "\x19Ethereum Signed Message:\n" + len + message
            let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
            keccak256([prefix.as_bytes(), message.as_ref()].concat())
        }
    };

    // 2) Recover and compare.
    let recovered = signature
        .recover_address_from_prehash(&prehash)
        .map_err(|e| anyhow!("recovery failed: {e}"))?;

    if recovered != expected {
        return Err(anyhow!("Recovered address {:#x} does not match expected address {:#x}", recovered, expected));
    }
    Ok(true)
}
