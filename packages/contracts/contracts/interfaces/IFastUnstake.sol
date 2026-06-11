// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IFastUnstake {
    function registerFastUnstake() external;
    function deregister() external;
    function control(uint32 erasToCheckPerBlock) external;

    // Head returns the BoundedVec of stashes currently being unstaked (batch).
    // For non-batched chains the array has 0 or 1 elements.
    function head() external view returns (address[] memory stashes);
    // Parallel array of deposit amounts, same order as head().
    function headDeposits() external view returns (uint256[] memory deposits);
    function queue(address stash) external view returns (uint256 deposit);
    function erasToCheckPerBlock() external view returns (uint32);
}
