// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {IERC1271} from "openzeppelin/contracts/interfaces/IERC1271.sol";
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {RiscZeroMockVerifier} from "risc0/test/RiscZeroMockVerifier.sol";
import {ControlID, RiscZeroGroth16Verifier} from "risc0/groth16/RiscZeroGroth16Verifier.sol";
import {SingleSign} from "../src/SingleSign.sol";
import {ImageID} from "../src/ImageID.sol";

/// @notice End-to-end test that verifies a real RISC Zero receipt produced by
///         `cargo run --bin generate_fixture` against a freshly-deployed
///         `RiscZeroGroth16Verifier`. When `RISC0_DEV_MODE=true` the fixture's
///         seal will be a fake-receipt seal accepted only by the matching
///         `RiscZeroMockVerifier(0xFFFFFFFF)`.
contract SingleSignE2ETest is Test {
    string internal constant FIXTURE_PATH = "artifacts/single_sign_fixture.json";

    bytes4 internal constant MOCK_SELECTOR = bytes4(0xFFFFFFFF);

    function test_FixtureProofIsValid() public {
        if (!vm.exists(FIXTURE_PATH)) {
            console2.log("fixture missing, run `cargo run --bin generate_fixture` to produce one");
            vm.skip(true);
        }

        string memory fixture = vm.readFile(FIXTURE_PATH);
        bytes32 imageId = vm.parseJsonBytes32(fixture, ".imageId");
        require(imageId == ImageID.SINGLE_SIGN_ID, "fixture imageId mismatches local SINGLE_SIGN_ID");

        address signer = vm.parseJsonAddress(fixture, ".signer");
        bytes32 hash = vm.parseJsonBytes32(fixture, ".hash");
        bytes memory seal = vm.parseJsonBytes(fixture, ".seal");

        bool fixtureIsMock = bytes4(seal) == MOCK_SELECTOR;
        bool envDevMode = vm.envOr("RISC0_DEV_MODE", false);
        if (fixtureIsMock != envDevMode) {
            console2.log(
                "fixture mode (mock=%s) does not match RISC0_DEV_MODE (%s); regenerate fixture in matching mode",
                fixtureIsMock,
                envDevMode
            );
            vm.skip(true);
        }

        IRiscZeroVerifier verifier = fixtureIsMock
            ? IRiscZeroVerifier(address(new RiscZeroMockVerifier(MOCK_SELECTOR)))
            : IRiscZeroVerifier(address(new RiscZeroGroth16Verifier(ControlID.CONTROL_ROOT, ControlID.BN254_CONTROL_ID)));

        SingleSign singleSign = new SingleSign(signer, verifier);

        bytes4 result = singleSign.isValidSignature(hash, seal);
        assertEq(result, IERC1271.isValidSignature.selector);
    }
}
