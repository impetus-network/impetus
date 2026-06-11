// SPDX-License-Identifier: Apache-2.0
pragma solidity >=0.8.3;

/// @dev Batch precompile at 0x0000000000000000000000000000000000000808.
/// Sub-calls execute with msg.sender = the original caller of the precompile
/// (i.e. value transfers and identity propagate from the outer call's caller,
/// not from the precompile address itself). The precompile never holds value.
/// gasLimit[i] = 0 means "forward all remaining gas".
interface Batch {
    /// Executes calls in order, continuing past any sub-call that reverts.
    /// Always returns to caller with success.
    function batchSome(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    /// Executes calls in order, stops on first revert but DOES NOT revert
    /// the outer call. Remaining indices are skipped.
    function batchSomeUntilFailure(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    /// Executes calls in order, reverts the entire batch if any sub-call reverts.
    /// Reverts of sub-calls bubble up with their original revert data.
    function batchAll(
        address[] memory to,
        uint256[] memory value,
        bytes[]   memory callData,
        uint64[]  memory gasLimit
    ) external;

    /// Emitted after each successful sub-call. `index` is the position in the
    /// input arrays (0-based). Non-indexed: lives in event data.
    event SubcallSucceeded(uint256 index);

    /// Emitted after each failed sub-call (only in batchSome / batchSomeUntilFailure).
    /// Non-indexed: lives in event data.
    event SubcallFailed(uint256 index);
}
