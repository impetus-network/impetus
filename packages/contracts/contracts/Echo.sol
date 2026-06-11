// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

contract Echo {
    uint256 public lastValue;
    address public lastSender;

    event Stored(uint256 value, address from);
    error Boom(uint256 reason);

    /// Reverts iff `x == 0`. Otherwise stores `x` and records `msg.sender`.
    function succeed(uint256 x) external {
        require(x != 0, "Echo: zero");
        lastValue = x;
        lastSender = msg.sender;
        emit Stored(x, msg.sender);
    }

    /// Reverts unconditionally with custom error `Boom(reason)`.
    function fail(uint256 reason) external pure {
        revert Boom(reason);
    }

    /// Plain native-value sink so batch transfers land.
    receive() external payable {}
}
