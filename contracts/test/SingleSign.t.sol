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

pragma solidity ^0.8.20;

import {Test} from "forge-std/Test.sol";
import {IERC1271} from "openzeppelin/contracts/interfaces/IERC1271.sol";
import {Receipt as RiscZeroReceipt} from "risc0/IRiscZeroVerifier.sol";
import {RiscZeroMockVerifier} from "risc0/test/RiscZeroMockVerifier.sol";
import {SingleSign} from "../src/SingleSign.sol";
import {ImageID} from "../src/ImageID.sol";

contract SingleSignTest is Test {
    bytes4 internal constant MOCK_SELECTOR = bytes4(0xFFFFFFFF);

    address public owner;
    SingleSign public singleSign;
    RiscZeroMockVerifier public verifier;

    function setUp() public {
        owner = makeAddr("owner");
        verifier = new RiscZeroMockVerifier(MOCK_SELECTOR);
        singleSign = new SingleSign(owner, verifier);
    }

    function test_RejectsProofWithMismatchedSigner() public {
        // Mock-prove a journal whose signer field is not the contract owner.
        // Contract recomputes the journal as abi.encode(owner, hash) and rejects.
        RiscZeroReceipt memory receipt = verifier.mockProve(
            ImageID.SINGLE_SIGN_ID,
            sha256(abi.encode(address(this), bytes32(0)))
        );

        bytes4 result = singleSign.isValidSignature(bytes32(0), receipt.seal);
        assertEq(result, bytes4(0));
    }

    function test_AcceptsProofForOwner() public {
        bytes32 hash = keccak256("digest under test");
        bytes memory journal = abi.encode(owner, hash);
        RiscZeroReceipt memory receipt = verifier.mockProve(ImageID.SINGLE_SIGN_ID, sha256(journal));

        bytes4 result = singleSign.isValidSignature(hash, receipt.seal);
        assertEq(result, IERC1271.isValidSignature.selector);
    }

    function test_RejectsProofForDifferentHash() public {
        bytes32 hash = keccak256("digest under test");
        bytes32 wrongHash = keccak256("a different digest");
        bytes memory journal = abi.encode(owner, hash);
        RiscZeroReceipt memory receipt = verifier.mockProve(ImageID.SINGLE_SIGN_ID, sha256(journal));

        bytes4 result = singleSign.isValidSignature(wrongHash, receipt.seal);
        assertEq(result, bytes4(0));
    }
}
