// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use risc0_zkvm::guest::env;
use common::{
    signing::{verify_signature, MessageMode},
    typed_data::verify_digest,
    Input, Output,
};

fn main() {
    // Read input from the host
    let input: Input = env::read();

    // Compute EIP-712 digest inside the guest from the JSON bytes
    let typed_data_slice =
        &input.typed_data_concat[input.digest_range.start..input.digest_range.end];
    let typed_data_digest = verify_digest(&String::from_utf8(typed_data_slice.to_vec()).unwrap())
        .expect("Invalid typed data");

    // Verify the signature against the same raw bytes using EIP-191 personal mode
    let _verified = verify_signature(
        input.typed_data_concat,
        input.signature,
        input.signer,
        MessageMode::Personal,
    )
    .expect("Invalid signature");

    // Groundwork only: commit (signer, digest) as the public output
    let output = Output {
        signer: input.signer,
        digest: typed_data_digest,
    };
    env::commit(&output);
}
