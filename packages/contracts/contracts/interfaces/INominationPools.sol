// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

struct BondExtraSource {
    uint8 kind; // 0=FreeBalance, 1=Rewards
    uint256 amount;
}

struct PoolRoleUpdate {
    uint8 op; // 0=Noop, 1=Set, 2=Remove
    address account;
}

// Input shape for `setCommission(uint32,(uint32,address))`. Two fields, in
// this order. Not used as a return value (the read-side
// commission tuple has more fields — see `PoolCommissionStatus` below).
struct PoolCommission {
    uint32 commission;
    address payee;
}

struct PoolCommissionChangeRate {
    uint32 maxIncrease;
    uint32 minDelay;
}

// Return shape for the trailing commission tuple of `bondedPools`. The
// precompile encodes 4 fields: current commission Perbill parts, max
// commission Perbill parts, the change-rate tuple, and the commission
// beneficiary. The static layout (no dynamic fields) is safe to nest inside
// the multi-value return tuple without a top-level offset pointer.
struct PoolCommissionStatus {
    uint32 current;
    uint32 max;
    PoolCommissionChangeRate changeRate;
    address payee;
}

// Each row of `PoolMember.unbondingEras`. The precompile returns
// `Vec<UnbondingEraSolidity>` here, i.e. `(uint32 era, uint256 points)` per
// row — not a bare `uint32[]`.
struct UnbondingEra {
    uint32 era;
    uint256 points;
}

interface INominationPools {
    function join(uint256 amount, uint32 poolId) external;
    function bondExtra(BondExtraSource calldata extra) external;
    function claimPayout() external;
    function unbond(address memberAccount, uint256 unbondingPoints) external;
    function poolWithdrawUnbonded(
        uint32 poolId,
        uint32 numSlashingSpans
    ) external;
    function withdrawUnbonded(
        address memberAccount,
        uint32 numSlashingSpans
    ) external;
    function create(
        uint256 amount,
        address root,
        address nominator,
        address bouncer
    ) external;
    function createWithPoolId(
        uint256 amount,
        address root,
        address nominator,
        address bouncer,
        uint32 poolId
    ) external;
    function nominate(uint32 poolId, address[] calldata validators) external;
    function setState(uint32 poolId, uint8 state) external;
    function setMetadata(uint32 poolId, bytes calldata metadata) external;
    function setConfigs(
        uint256 minJoinBond,
        uint256 minCreateBond,
        uint32 maxPools,
        uint32 maxMembers,
        uint32 maxMembersPerPool,
        uint32 globalMaxCommission
    ) external;
    // NOTE: 3 POSITIONAL tuples, not a [3] array (matches precompile selector
    // updateRoles(uint32,(uint8,address),(uint8,address),(uint8,address)))
    function updateRoles(
        uint32 poolId,
        PoolRoleUpdate calldata root,
        PoolRoleUpdate calldata nominator,
        PoolRoleUpdate calldata bouncer
    ) external;
    function chill(uint32 poolId) external;
    function bondExtraOther(
        address member,
        BondExtraSource calldata extra
    ) external;
    function setCommission(
        uint32 poolId,
        PoolCommission calldata commission
    ) external;
    function setCommissionMax(uint32 poolId, uint32 maxCommission) external;
    function setCommissionChangeRate(
        uint32 poolId,
        PoolCommissionChangeRate calldata changeRate
    ) external;
    function claimCommission(uint32 poolId) external;

    function bondedPools(uint32 poolId) external view returns (
        uint256 points,
        uint8 state,
        uint32 memberCounter,
        address[] memory roles, // Always length 3: [root, nominator, bouncer]
        PoolCommissionStatus memory commission
    );
    function poolMembers(address account) external view returns (
        uint32 poolId,
        uint256 points,
        uint256 lastRecordedRewardCounter,
        UnbondingEra[] memory unbondingEras
    );
    function metadata(uint32 poolId) external view returns (bytes memory);
    function lastPoolId() external view returns (uint32);
}
