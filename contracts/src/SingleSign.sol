// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.20;

import {IERC20} from "openzeppelin/contracts/interfaces/IERC20.sol";
import {IERC1271} from "openzeppelin/contracts/interfaces/IERC1271.sol";
import {Ownable} from "openzeppelin/contracts/access/Ownable.sol";

import {IPermit2} from "permit2/interfaces/IPermit2.sol";

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol";

// @title SingleSign
// @author Kai Aldag<kai@eggtech.io>
// @notice Allows a single signature to be used for many EIP-712 typed data signatures verified using EIP-1271
contract SingleSign is IERC1271, Ownable {
    IRiscZeroVerifier public immutable VERIFIER;
    bytes32 public constant IMAGE_ID = ImageID.SINGLE_SIGN_ID;

    constructor(address _owner, IRiscZeroVerifier _verifier) Ownable(_owner) {
        VERIFIER = _verifier;
    }

    function isValidSignature(
        bytes32 hash,
        bytes calldata signature
    ) external view returns (bytes4) {
        bytes memory journal = abi.encode(owner(), hash);

        try VERIFIER.verify(signature, IMAGE_ID, sha256(journal)) {
            return IERC1271.isValidSignature.selector;
        } catch {
            return bytes4(0);
        }
    }

    function approve(address token, uint256 amount) external onlyOwner {
        IERC20(token).approve(address(this), amount);
    }

    function transfer(address token, address to, uint256 amount) external onlyOwner {
        IERC20(token).transfer(to, amount);
    }
}
