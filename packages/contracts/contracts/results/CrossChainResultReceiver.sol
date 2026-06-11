// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IResultSource, LotteryResult} from "../interfaces/IResultSource.sol";
import {IResultVerifier} from "../interfaces/IResultVerifier.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract CrossChainResultReceiver is IResultSource, Ownable {
    IResultVerifier public verifier;

    mapping(uint256 roundId => LotteryResult) internal results;
    mapping(uint256 roundId => bool) public requested;
    mapping(uint256 roundId => bool) public fulfilled;

    error RoundAlreadyRequested(uint256 roundId);
    error RoundNotRequested(uint256 roundId);
    error RoundAlreadyFulfilled(uint256 roundId);
    error RoundNotFulfilled(uint256 roundId);
    error InvalidProof();

    constructor(address _verifier) Ownable(msg.sender) {
        verifier = IResultVerifier(_verifier);
    }

    function requestResult(uint256 roundId) external {
        if (requested[roundId]) revert RoundAlreadyRequested(roundId);
        requested[roundId] = true;
        emit ResultRequested(roundId);
    }

    function submitResult(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external {
        if (!requested[roundId]) revert RoundNotRequested(roundId);
        if (fulfilled[roundId]) revert RoundAlreadyFulfilled(roundId);
        if (!verifier.verify(roundId, result, proof)) revert InvalidProof();

        results[roundId] = result;
        fulfilled[roundId] = true;
        emit ResultFulfilled(roundId);
    }

    function getResult(uint256 roundId) external view returns (LotteryResult memory) {
        if (!fulfilled[roundId]) revert RoundNotFulfilled(roundId);
        return results[roundId];
    }

    function hasResult(uint256 roundId) external view returns (bool) {
        return fulfilled[roundId];
    }

    function setVerifier(IResultVerifier _verifier) external onlyOwner {
        verifier = _verifier;
    }
}
