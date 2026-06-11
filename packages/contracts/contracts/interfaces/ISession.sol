// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ISession {
    function setKeys(bytes calldata keys, bytes calldata proof) external;
    function purgeKeys() external;
    function currentIndex() external view returns (uint32);
    function nextKeys(address validator) external view returns (bytes memory);
    function queuedKeys()
        external
        view
        returns (address[] memory validators, bytes[] memory keys);
}
