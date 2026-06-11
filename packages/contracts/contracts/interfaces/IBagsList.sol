// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IBagsList {
    function putInFrontOf(address lighter) external;
    function rebag(address dislocated) external;

    function listSize() external view returns (uint32);
    function score(address who) external view returns (uint64);
    function bagOf(address who) external view returns (uint64);
}
