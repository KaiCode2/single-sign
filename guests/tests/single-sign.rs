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

use alloy_primitives::{Address, Bytes, B256};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolValue;
use common::{find_concatenated_json_ranges, typed_data::verify_digest, DigestRange, Input, Output};
use guests::SINGLE_SIGN_ELF;
use risc0_zkvm::{default_executor, ExecutorEnv};

const SAMPLE_TWO_PERMIT: &str = include_str!("../../examples/typed_data_permit2_two.json");

fn random_signer_input(target_index: usize) -> (Input, Address, B256) {
    let signer = PrivateKeySigner::random();
    let signer_address = signer.address();

    let typed_data_concat: Bytes = Bytes::from(SAMPLE_TWO_PERMIT.as_bytes().to_vec());
    let ranges = find_concatenated_json_ranges(SAMPLE_TWO_PERMIT).expect("ranges parse");
    let range: DigestRange = ranges[target_index].clone();

    let signature = signer
        .sign_message_sync(&typed_data_concat)
        .expect("sign message");

    let expected_digest = verify_digest(&SAMPLE_TWO_PERMIT[range.start..range.end])
        .expect("compute eip712 digest");

    let input = Input {
        signer: signer_address,
        signature,
        typed_data_concat,
        digest_range: range,
    };

    (input, signer_address, expected_digest)
}

fn run_guest(input: &Input) -> Output {
    let env = ExecutorEnv::builder()
        .write(input)
        .unwrap()
        .build()
        .unwrap();
    let session_info = default_executor().execute(env, SINGLE_SIGN_ELF).unwrap();
    Output::abi_decode(&session_info.journal.bytes).expect("journal abi-decodes to Output")
}

#[test]
fn commits_signer_and_digest_for_first_message() {
    let (input, signer, expected_digest) = random_signer_input(0);
    let output = run_guest(&input);
    assert_eq!(output.signer, signer);
    assert_eq!(output.digest, expected_digest);
}

#[test]
fn commits_signer_and_digest_for_second_message() {
    let (input, signer, expected_digest) = random_signer_input(1);
    let output = run_guest(&input);
    assert_eq!(output.signer, signer);
    assert_eq!(output.digest, expected_digest);
}

#[test]
#[should_panic(expected = "Invalid signature")]
fn rejects_signature_from_a_different_signer() {
    let real_signer = PrivateKeySigner::random();
    let other_signer = PrivateKeySigner::random();

    let typed_data_concat: Bytes = Bytes::from(SAMPLE_TWO_PERMIT.as_bytes().to_vec());
    let ranges = find_concatenated_json_ranges(SAMPLE_TWO_PERMIT).unwrap();
    let signature = other_signer.sign_message_sync(&typed_data_concat).unwrap();

    let input = Input {
        signer: real_signer.address(),
        signature,
        typed_data_concat,
        digest_range: ranges[0].clone(),
    };

    run_guest(&input);
}
