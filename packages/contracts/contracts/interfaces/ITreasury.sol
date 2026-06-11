// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ITreasury {
    /// Root-only (must route via Sudo::sudo). Direct caller must be sudo key.
    function spendLocal(uint256 amount, address beneficiary) external;
    /// Permissionless. Claim a previously approved spend.
    function payout(uint32 index) external;
    /// Root-only. Cancel an approved spend.
    function voidSpend(uint32 index) external;
    /// Permissionless. Inspect / clean up an expired spend.
    function checkStatus(uint32 index) external;

    function pot() external view returns (uint256);
    function spendCount() external view returns (uint32);
    function approvals() external view returns (uint32[] memory);
}
