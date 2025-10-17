## Aggregated ERC‑712 Validator (Typed Data Concatenation with Inclusion Proofs)

This project implements an aggregated validator for EIP‑712 typed data using zk-SNARKs. The user signs once over the concatenation of many typed‑data payloads, and then per‑digest proofs attest that a specific digest comes from a slice of that signed payload.

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

### Why this approach?

- **Clear signing**: The user signs exactly the concatenation of human‑readable typed data, not a Merkle root.
- **Aggregated authorization**: One signature covers many typed‑data messages across dapps and chains.
- **ZK inclusion proofs**: Each per‑message proof shows that the verified digest derives from a slice of the signed payload.

---

## Repository Layout

```text
single-sign
├── Cargo.toml                     # Workspace (host, methods, single_sign_types)
├── host/                          # Host: builds inputs, runs the prover
│   └── src/main.rs
├── methods/                       # Guest program (zkVM)
│   ├── build.rs
│   ├── guest/src/main.rs          # Verifies signature + computes EIP‑712 digest of slice
│   └── src/lib.rs                 # Exposes SINGLE_SIGN_ELF & SINGLE_SIGN_ID
└── single_sign_types/             # Shared types + EIP‑712 helpers
    └── src/{lib.rs,typed_data.rs,signing.rs}
```

Notable pieces:

- `single_sign_types::typed_data::verify_digest` parses EIP‑712 JSON and computes its digest.
- `single_sign_types::signing::verify_signature` checks an EOA signature against the full concatenation.
- `host/src/main.rs` currently demonstrates three sample Permit2 `PermitTransferFrom` messages, builds compact JSON for each, concatenates them, signs once, and proves/prints `(signer, digest)` per message.

---

## Quick Start

Prerequisites: install `rustup` (see `https://rustup.rs`). The pinned toolchain in `rust-toolchain.toml` will be automatically used.

Build and run (host will compile and execute the guest):

```bash
cargo run
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
