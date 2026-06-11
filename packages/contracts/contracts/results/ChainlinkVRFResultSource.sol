// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {VRFConsumerBaseV2Plus} from "@chainlink/contracts/src/v0.8/vrf/dev/VRFConsumerBaseV2Plus.sol";
import {VRFV2PlusClient} from "@chainlink/contracts/src/v0.8/vrf/dev/libraries/VRFV2PlusClient.sol";
import {IResultSource, LotteryResult} from "../interfaces/IResultSource.sol";

contract ChainlinkVRFResultSource is VRFConsumerBaseV2Plus, IResultSource {
    bytes32 public immutable i_keyHash;
    uint256 public immutable i_subscriptionId;
    uint16 public constant REQUEST_CONFIRMATIONS = 3;
    uint32 public constant CALLBACK_GAS_LIMIT = 300_000;
    uint32 public constant NUM_WORDS = 2;

    mapping(uint256 roundId => uint256 requestId) public roundToRequest;
    mapping(uint256 requestId => uint256 roundId) public requestToRound;
    mapping(uint256 roundId => LotteryResult) internal _results;
    mapping(uint256 roundId => bool) public fulfilled;

    error RoundAlreadyRequested(uint256 roundId);
    error RoundNotFulfilled(uint256 roundId);

    constructor(
        address vrfCoordinator,
        bytes32 keyHash,
        uint256 subscriptionId
    ) VRFConsumerBaseV2Plus(vrfCoordinator) {
        i_keyHash = keyHash;
        i_subscriptionId = subscriptionId;
    }

    function requestResult(uint256 roundId) external {
        if (roundToRequest[roundId] != 0) revert RoundAlreadyRequested(roundId);

        uint256 requestId = s_vrfCoordinator.requestRandomWords(
            VRFV2PlusClient.RandomWordsRequest({
                keyHash: i_keyHash,
                subId: i_subscriptionId,
                requestConfirmations: REQUEST_CONFIRMATIONS,
                callbackGasLimit: CALLBACK_GAS_LIMIT,
                numWords: NUM_WORDS,
                extraArgs: VRFV2PlusClient._argsToBytes(
                    VRFV2PlusClient.ExtraArgsV1({nativePayment: false})
                )
            })
        );

        roundToRequest[roundId] = requestId;
        requestToRound[requestId] = roundId;
        emit ResultRequested(roundId);
    }

    function fulfillRandomWords(
        uint256 requestId,
        uint256[] calldata randomWords
    ) internal override {
        uint256 roundId = requestToRound[requestId];
        if (roundId == 0) return;

        uint8 specialPrize = uint8(
            uint256(keccak256(abi.encode(randomWords[0], uint256(0)))) % 100
        );

        uint8[] memory allPrizes = new uint8[](27);
        allPrizes[0] = specialPrize;
        for (uint256 i = 1; i < 27; ) {
            allPrizes[i] = uint8(
                uint256(keccak256(abi.encode(randomWords[0], randomWords[1], i))) % 100
            );
            unchecked { ++i; }
        }

        _results[roundId] = LotteryResult({
            specialPrize: specialPrize,
            allPrizes: allPrizes
        });
        fulfilled[roundId] = true;

        emit ResultFulfilled(roundId);
    }

    function getResult(uint256 roundId) external view returns (LotteryResult memory) {
        if (!fulfilled[roundId]) revert RoundNotFulfilled(roundId);
        return _results[roundId];
    }

    function hasResult(uint256 roundId) external view returns (bool) {
        return fulfilled[roundId];
    }
}
