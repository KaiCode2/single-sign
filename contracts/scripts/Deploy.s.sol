// Copyright 2025 RISC Zero, Inc.
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

import {Script, console2} from "forge-std/Script.sol";
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {SingleSign} from "../src/SingleSign.sol";
import {LibString} from "solady/utils/LibString.sol";

contract Deploy is Script {
    using LibString for string;

    function run() external {
        // load ENV variables first
        uint256 ownerKey = vm.envUint("USER_PRIVATE_KEY");
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");
        address ownerAddress = vm.addr(ownerKey);
        address deployerAddress = vm.addr(deployerKey);
        address verifierAddress = getVerifierAddress();
        vm.startBroadcast(deployerKey);

        IRiscZeroVerifier verifier = IRiscZeroVerifier(verifierAddress);
        SingleSign singleSign = new SingleSign{salt: bytes32(bytes20(ownerAddress))}(
            ownerAddress,
            verifier
        );
        address singleSignAddress = address(singleSign);
        console2.log("Deployed SingleSign to", singleSignAddress);

        vm.stopBroadcast();
    }

    function getVerifierAddress() internal returns (address verifierAddress) {
        string memory configPath = vm.projectRoot().concat(
            "/contracts/configs/verifiers.toml"
        );
        string memory toml = vm.readFile(configPath);

        string memory chainIdStr = vm.toString(block.chainid);
        string memory path = string.concat(
            '.chains["',
            chainIdStr,
            '"].groth16'
        );

        bytes memory verifierBytes = vm.parseToml(toml, path);

        (verifierAddress) = abi.decode(verifierBytes, (address));
    }
}
