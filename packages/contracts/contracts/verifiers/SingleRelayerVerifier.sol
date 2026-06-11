// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IResultVerifier} from "../interfaces/IResultVerifier.sol";
import {LotteryResult} from "../interfaces/IResultSource.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

contract SingleRelayerVerifier is IResultVerifier {
    address public immutable trustedRelayer;

    error ZeroAddress();

    constructor(address _trustedRelayer) {
        if (_trustedRelayer == address(0)) revert ZeroAddress();
        trustedRelayer = _trustedRelayer;
    }

    function verify(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external view returns (bool) {
        bytes32 hash = keccak256(
            abi.encode(roundId, result.specialPrize, result.allPrizes)
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(hash);
        address signer = ECDSA.recover(ethSignedHash, proof);
        return signer == trustedRelayer;
    }
}
