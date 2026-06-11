// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct LotteryResult {
    uint8 specialPrize;
    uint8[] allPrizes;
}

interface IResultSource {
    event ResultRequested(uint256 indexed roundId);
    event ResultFulfilled(uint256 indexed roundId);

    function requestResult(uint256 roundId) external;
    function getResult(uint256 roundId) external view returns (LotteryResult memory);
    function hasResult(uint256 roundId) external view returns (bool);
}
