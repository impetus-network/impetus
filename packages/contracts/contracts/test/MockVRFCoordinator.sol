// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {VRFV2PlusClient} from "@chainlink/contracts/src/v0.8/vrf/dev/libraries/VRFV2PlusClient.sol";
import {IVRFCoordinatorV2Plus} from "@chainlink/contracts/src/v0.8/vrf/dev/interfaces/IVRFCoordinatorV2Plus.sol";

contract MockVRFCoordinator is IVRFCoordinatorV2Plus {
    uint256 public nextRequestId = 1;
    mapping(uint256 => address) public consumers;

    function requestRandomWords(
        VRFV2PlusClient.RandomWordsRequest calldata /* req */
    ) external override returns (uint256 requestId) {
        requestId = nextRequestId++;
        consumers[requestId] = msg.sender;
    }

    function fulfillRequest(uint256 requestId, uint256[] calldata randomWords) external {
        address consumer = consumers[requestId];
        require(consumer != address(0), "unknown request");
        (bool success, ) = consumer.call(
            abi.encodeWithSignature(
                "rawFulfillRandomWords(uint256,uint256[])",
                requestId,
                randomWords
            )
        );
        require(success, "fulfill failed");
    }

    // Stub implementations for IVRFSubscriptionV2Plus

    function addConsumer(uint256, address) external pure override {}

    function removeConsumer(uint256, address) external pure override {}

    function cancelSubscription(uint256, address) external pure override {}

    function acceptSubscriptionOwnerTransfer(uint256) external pure override {}

    function requestSubscriptionOwnerTransfer(uint256, address) external pure override {}

    function createSubscription() external pure override returns (uint256) {
        return 1;
    }

    function getSubscription(
        uint256
    )
        external
        pure
        override
        returns (uint96, uint96, uint64, address, address[] memory)
    {
        address[] memory empty = new address[](0);
        return (0, 0, 0, address(0), empty);
    }

    function pendingRequestExists(uint256) external pure override returns (bool) {
        return false;
    }

    function getActiveSubscriptionIds(
        uint256,
        uint256
    ) external pure override returns (uint256[] memory) {
        return new uint256[](0);
    }

    function fundSubscriptionWithNative(uint256) external payable override {}
}
