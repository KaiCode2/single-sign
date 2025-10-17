use risc0_zkvm::guest::env;
use single_sign_types::{
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
    let verified = verify_signature(
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
