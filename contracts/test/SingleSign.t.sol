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
import {RiscZeroCheats} from "risc0/test/RiscZeroCheats.sol";
import {Receipt as RiscZeroReceipt} from "risc0/IRiscZeroVerifier.sol";
import {RiscZeroMockVerifier} from "risc0/test/RiscZeroMockVerifier.sol";
import {VerificationFailed} from "risc0/IRiscZeroVerifier.sol";
import {SingleSign} from "../src/SingleSign.sol";
import {ImageID} from "../src/ImageID.sol";

contract SingleSignTest is RiscZeroCheats, Test {
    address public owner;
    SingleSign public singleSign;
    RiscZeroMockVerifier public verifier;

    function setUp() public {
        owner = makeAddr("owner");
        verifier = new RiscZeroMockVerifier(0);
        singleSign = new SingleSign(owner, verifier);
    }

    // Try using a proof for the evenness of 4 to set 1 on the contract.
    function test_RejectInvalidProof() public {
        RiscZeroReceipt memory receipt = verifier.mockProve(
            ImageID.SINGLE_SIGN_ID,
            sha256(abi.encode(address(this), bytes32(0)))
        );

        bytes4 result = singleSign.isValidSignature(bytes32(0), receipt.seal);
        assertEq(result, bytes4(0));
    }
}
