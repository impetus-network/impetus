// ABI for the Nomination Pools precompile (INominationPools.sol) at 0x0820.
// Member + owner subset: join/claim/unbond/withdraw, create, nominate, chill,
// plus the read views needed to render pools and a member's position.
export const NominationPoolsAbi = [
  {
    type: "function",
    name: "join",
    stateMutability: "nonpayable",
    inputs: [
      { name: "amount", type: "uint256" },
      { name: "poolId", type: "uint32" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "bondExtra",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "extra",
        type: "tuple",
        components: [
          { name: "kind", type: "uint8" },
          { name: "amount", type: "uint256" },
        ],
      },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "claimPayout",
    stateMutability: "nonpayable",
    inputs: [],
    outputs: [],
  },
  {
    type: "function",
    name: "unbond",
    stateMutability: "nonpayable",
    inputs: [
      { name: "memberAccount", type: "address" },
      { name: "unbondingPoints", type: "uint256" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "withdrawUnbonded",
    stateMutability: "nonpayable",
    inputs: [
      { name: "memberAccount", type: "address" },
      { name: "numSlashingSpans", type: "uint32" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "create",
    stateMutability: "nonpayable",
    inputs: [
      { name: "amount", type: "uint256" },
      { name: "root", type: "address" },
      { name: "nominator", type: "address" },
      { name: "bouncer", type: "address" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "nominate",
    stateMutability: "nonpayable",
    inputs: [
      { name: "poolId", type: "uint32" },
      { name: "validators", type: "address[]" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "chill",
    stateMutability: "nonpayable",
    inputs: [{ name: "poolId", type: "uint32" }],
    outputs: [],
  },
  {
    type: "function",
    name: "claimCommission",
    stateMutability: "nonpayable",
    inputs: [{ name: "poolId", type: "uint32" }],
    outputs: [],
  },
  {
    type: "function",
    name: "bondedPools",
    stateMutability: "view",
    inputs: [{ name: "poolId", type: "uint32" }],
    outputs: [
      { name: "points", type: "uint256" },
      { name: "state", type: "uint8" },
      { name: "memberCounter", type: "uint32" },
      { name: "roles", type: "address[]" },
      {
        name: "commission",
        type: "tuple",
        components: [
          { name: "current", type: "uint32" },
          { name: "max", type: "uint32" },
          {
            name: "changeRate",
            type: "tuple",
            components: [
              { name: "maxIncrease", type: "uint32" },
              { name: "minDelay", type: "uint32" },
            ],
          },
          { name: "payee", type: "address" },
        ],
      },
    ],
  },
  {
    type: "function",
    name: "poolMembers",
    stateMutability: "view",
    inputs: [{ name: "account", type: "address" }],
    outputs: [
      { name: "poolId", type: "uint32" },
      { name: "points", type: "uint256" },
      { name: "lastRecordedRewardCounter", type: "uint256" },
      {
        name: "unbondingEras",
        type: "tuple[]",
        components: [
          { name: "era", type: "uint32" },
          { name: "points", type: "uint256" },
        ],
      },
    ],
  },
  {
    type: "function",
    name: "metadata",
    stateMutability: "view",
    inputs: [{ name: "poolId", type: "uint32" }],
    outputs: [{ name: "", type: "bytes" }],
  },
  {
    type: "function",
    name: "lastPoolId",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
  },
] as const;

// pallet-nomination-pools PoolState.
export const POOL_STATE_LABELS = ["Open", "Blocked", "Destroying"] as const;

// BondExtraSource.kind values.
export const BOND_EXTRA_FREE_BALANCE = 0;
export const BOND_EXTRA_REWARDS = 1;
