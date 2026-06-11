// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {LotteryResult} from "./IResultSource.sol";

interface IResultVerifier {
    function verify(
        uint256 roundId,
        LotteryResult calldata result,
        bytes calldata proof
    ) external view returns (bool);
}
