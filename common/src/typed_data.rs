use alloy_primitives::B256;
use alloy_dyn_abi::TypedData;
use anyhow::{anyhow, Result};

/// Compute a generic EIP-712 digest for any compliant typed-data JSON.
/// Input is a JSON string with `types`, `primaryType`, `domain`, and `message`.
/// Returns the bytes32 digest: keccak256("\x19\x01" || domainSeparator || hashStruct(message)).
pub fn verify_digest(typed_data_json: &str) -> Result<B256> {
    let typed: TypedData = serde_json::from_str(typed_data_json)
        .map_err(|e| anyhow!("Invalid EIP-712 typed data JSON: {e}"))?;
    typed
        .eip712_signing_hash()
        .map_err(|e| anyhow!("Failed computing EIP-712 digest: {e}"))
}
