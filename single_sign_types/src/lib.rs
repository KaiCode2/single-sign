pub mod signing;
pub mod typed_data;

use alloy_primitives::{Address, Bytes, Signature, B256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub signer: Address,
    pub signature: Signature,
    pub typed_data_concat: Bytes,
    pub digest_range: DigestRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub signer: Address,
    pub digest: B256,
}
