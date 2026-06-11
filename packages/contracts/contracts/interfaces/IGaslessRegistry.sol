// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IGaslessRegistry {
    event RuleSet(
        address indexed contract_,
        bytes4 indexed selector,
        bool enabled,
        uint256 minValue
    );
    event RuleRemoved(address indexed contract_, bytes4 indexed selector);

    function getRule(
        address contract_,
        bytes4 selector
    ) external view returns (bool enabled, uint256 minValue);

    function isGasless(
        address contract_,
        bytes calldata input,
        uint256 value,
        uint256 gasLimit
    ) external view returns (bool);

    function setRule(
        address contract_,
        bytes4 selector,
        uint256 minValue,
        bool enabled
    ) external;

    function removeRule(address contract_, bytes4 selector) external;
}
