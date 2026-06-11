// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct RewardDestination {
    uint8 kind; // 0=Staked, 1=Stash, 2=Controller, 3=Account, 4=None
    address account;
}

struct ValidatorPrefs {
    uint16 commissionPercent; // 0-100 (validated by precompile to reject >100)
    bool blocked;
}

struct UnlockChunk {
    uint32 era;
    uint256 value;
}

interface IStaking {
    function bond(uint256 value, RewardDestination calldata payee) external;
    function bondExtra(uint256 maxAdditional) external;
    function unbond(uint256 value) external;
    function withdrawUnbonded(uint32 numSlashingSpans) external;
    function validate(ValidatorPrefs calldata prefs) external;
    function nominate(address[] calldata targets) external;
    function chill() external;
    function setPayee(RewardDestination calldata payee) external;
    function payoutStakers(address validatorStash, uint32 era) external;
    function payoutStakersByPage(
        address validatorStash,
        uint32 era,
        uint32 page
    ) external;
    function rebond(uint256 value) external;
    function kick(address[] calldata who) external;
    function chillOther(address controller) external;
    function forceApplyMinCommission(address validator) external;
    function reapStash(address stash, uint32 numSlashingSpans) external;

    function currentEra() external view returns (uint32);
    function activeEra() external view returns (uint32 index, uint64 startMs);
    function minNominatorBond() external view returns (uint256);
    function minValidatorBond() external view returns (uint256);
    function validatorCount() external view returns (uint32);
    function bonded(address controller) external view returns (address);
    function ledger(
        address controller
    ) external view returns (uint256 active, uint256 total, UnlockChunk[] memory unlocking);
    function nominators(
        address stash
    ) external view returns (address[] memory targets, uint32 submittedIn, bool suppressed);
    function validators(
        address stash
    ) external view returns (uint16 commissionPercent, bool blocked);
    function minActiveStake() external view returns (uint256);
    function counterForValidators() external view returns (uint32);
    function counterForNominators() external view returns (uint32);
    function historyDepth() external view returns (uint32);
    function erasValidatorReward(uint32 era) external view returns (uint256);
}
