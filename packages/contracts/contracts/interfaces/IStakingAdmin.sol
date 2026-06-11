// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IStakingAdmin {
    function setValidatorCount(uint32 newCount) external;
    function increaseValidatorCount(uint32 additional) external;
    function scaleValidatorCount(uint8 factorPercent) external;
    function setInvulnerables(address[] calldata validators) external;
    function forceUnstake(address stash, uint32 numSlashingSpans) external;
    function forceNewEra() external;
    function forceNoEras() external;
    function forceNewEraAlways() external;
    function cancelDeferredSlash(
        uint32 era,
        uint32[] calldata slashIndices
    ) external;
    function setStakingConfigs(
        uint256 minNominatorBond,
        uint256 minValidatorBond,
        uint32 maxNominatorCount,
        uint32 maxValidatorCount,
        uint8 chillThresholdPercent,
        uint32 minCommission
    ) external;
    function chillOther(address stash) external;
}
