// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract DelegateProxy {
    function delegate(address target, bytes calldata data) external returns (bool, bytes memory) {
        return target.delegatecall(data);
    }
}
