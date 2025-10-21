## Aggregated ERC‑712 Validator (Typed Data Concatenation with Inclusion Proofs)

This project implements an aggregated validator for [EIP‑712](https://eips.ethereum.org/EIPS/eip-712) typed data using zk-SNARKs powered by [RISC Zero](https://risczero.com/). The user signs once over the concatenation of many typed‑data payloads, and then per‑digest proofs attest that a specific digest comes from a slice of that signed payload.

At a high level:

- The host builds many EIP‑712 typed‑data JSONs, compacts them, concatenates them into one `typed_data_concat` string, and signs that single string once with the user's EOA key.
- For each message to verify, we compute the byte range `[start, end)` of that message's compact JSON within `typed_data_concat` and invoke the zkVM with:
  - `signer` (EOA address),
  - `signature` (over the full `typed_data_concat`),
  - `typed_data_concat` (bytes),
  - `digest_range` (`start`, `end`).
- Inside the guest, we:
  - Verify the signature over the full `typed_data_concat`.
  - Re‑compute the EIP‑712 digest of the slice `typed_data_concat[start..end]` and commit `(signer, digest)` as the public journal output.
- The host obtains a RISC Zero receipt per digest. Anyone can verify the receipt against the program image ID and read `(signer, digest)` from the journal.

This enables "sign once, prove many" UX for flows like Permit2 where multiple independent EIP‑712 messages would otherwise require separate user signatures.

Note: Smart contracts in `contracts/` are developed and built with Foundry. Use `forge build` to compile contracts before building the Rust components.

### Why this approach?

- **Clear signing**: The user signs exactly the concatenation of human‑readable typed data, not a Merkle root.
- **Aggregated authorization**: One signature covers many typed‑data messages across dapps and chains.
- **ZK inclusion proofs**: Each per‑message proof shows that the verified digest derives from a slice of the signed payload.

---

## Repository Layout

```text
single-sign
├── Cargo.toml                     # Workspace (apps, guests, common)
├── foundry.toml                   # Foundry config for contracts
├── apps/                          # CLI binaries
│   └── src/bin/{prove.rs,sign_file.rs}
├── common/                        # Shared types + helpers
│   └── src/{lib.rs,typed_data.rs,signing.rs}
├── guests/                        # RISC Zero guest program(s)
│   ├── Cargo.toml
│   ├── src/lib.rs                 # include!(…/methods.rs) for built methods
│   └── single-sign/src/main.rs    # Verifies signature + computes digest of slice
├── contracts/                     # Solidity contracts (Foundry)
│   ├── src/
│   ├── test/
│   └── scripts/
└── examples/
    └── typed_data_concat.json
```

Notable pieces:

- `common::typed_data::verify_digest` parses EIP‑712 JSON and computes its digest.
- `common::signing::verify_signature` checks an EOA signature against the concatenation (EIP‑191 personal mode by default).
- `apps/src/bin/prove.rs` loads a concatenated typed‑data file, finds slice ranges, proves each inside the zkVM, and prints `(signer, digest)`.
- `apps/src/bin/sign_file.rs` signs arbitrary file bytes and prints the digest, signature, and signer.
- `guests/single-sign/src/main.rs` runs inside the zkVM and commits `(signer, digest)` to the journal.

---

## Quick Start

Prerequisites: install `rustup` (see [rustup.rs](https://rustup.rs)) and Foundry (`forge`) (see the [Foundry installation guide](https://book.getfoundry.sh/getting-started/installation)). The pinned Rust toolchain in `rust-toolchain.toml` will be used automatically.

Build the contracts (Foundry) and the Rust workspace:

```bash
forge build
cargo build
```

Run the host example:

```bash
cargo run
```

### Sample commands

- Sign a file's raw bytes and print digest, signature, and signer:

```bash
cargo run --bin sign_file -- --file-path ./stash/typed_data_concat.json
```

- Prove inclusion for each concatenated typed‑data JSON slice and print `(signer, digest)`:

```bash
cargo run -- --file-path examples/typed_data_concat.json \
  --signer 0x91738e8f069208baa0efe5441c6d0a9f0b9e27f2 \
  --signature 0x04a482be75182d5bcb450c039d767cf8e24e3aeb35789a89dd7490ca2828b37468b6b081a55c567c0629df2180c5ca7758c04e3880bf76f2ccb38af3e493bea51c \
  --rpc-url "https://ethereum-sepolia-rpc.publicnode.com" \
  --private-key <hex-or-env> \
  --account-address 0x5989b7E895D7f1bED932A82bEB40eB93264C787B
```

For faster local iteration, enable dev‑mode and optional execution logs:

```bash
RISC0_DEV_MODE=1 RUST_LOG=info cargo run
```

What you'll see:

- The host prints the EIP‑712 digests for each sample Permit2 message.
- It computes byte ranges for each compact JSON within the concatenation.
- For each range, it proves inside the zkVM and prints the guest output `(signer, digest)`, then verifies the receipt against `SINGLE_SIGN_ID`.

---

## Customizing for Your Typed Data

1) Build your EIP‑712 JSONs in the host. The sample code uses Permit2 structs, but you can construct any typed data as long as it conforms to EIP‑712. Ensure the JSON you pass to the guest is exactly the JSON you signed.

2) Compact and concatenate deterministically. The demo removes spaces and newlines before concatenation to make ranges stable:

- Use the same compaction when computing ranges and when preparing the exact bytes to sign.
- Compute `[start, end)` for each message's compact JSON within the concatenated string.

3) Sign once over the full concatenation. The demo uses an in‑memory random key; in production, use a real EOA signer.

4) Prove per message. For each range, call the zkVM with `Input { signer, signature, typed_data_concat, digest_range }` and obtain a receipt committing `(signer, digest)`.

If you need to change signature semantics (e.g., EIP‑712 typed‑data signing vs EIP‑191 personal), update both the host signing method and `single_sign_types::signing::verify_signature` mode accordingly so they match.

---

## On‑Chain Verification Sketch

RISC Zero receipts can be verified on‑chain by checking the program image ID and feeding the journal as public input to your verifier. Conceptually:

```solidity
// Pseudocode: adapt to your target verifier and encoding
contract AccountLike1271 {
    bytes32 public immutable imageId;      // must match the compiled guest image (SINGLE_SIGN_ID)
    address public immutable signer;       // authorized EOA
    IRisc0Verifier public verifier;        // chain-specific verifier

    constructor(bytes32 _imageId, address _signer, IRisc0Verifier _verifier) {
        imageId = _imageId;
        signer = _signer;
        verifier = _verifier;
    }

    function isValidSignature(bytes32 digest, bytes calldata proof)
        external
        view
        returns (bytes4)
    {
        // Journal encodes `(signer, digest)`; adjust if your encoding differs.
        bytes memory journal = abi.encodePacked(signer, digest);
        bool ok = verifier.verify(imageId, proof, journal);
        return ok ? IERC1271.isValidSignature.selector : bytes4(0xffffffff);
    }
}
```

Notes:

- Ensure the journal encoding on‑chain matches the guest's committed output `(signer, digest)`.
- The `imageId` must match `methods::SINGLE_SIGN_ID` from this repo's build.

---

## Development Tips

- Determinism matters: any discrepancy between the bytes you sign and the bytes the guest sees will invalidate proofs. Keep compaction and ordering identical.
- Logging: run with `RUST_LOG=info` to see progress and ranges.
- Remote proving: you can integrate with Bonsai to offload proving. Example env:

```bash
BONSAI_API_KEY="…" BONSAI_API_URL="…" cargo run
```

---

## License

This repository is licensed under the terms of the MIT license. See `LICENSE` for details.
