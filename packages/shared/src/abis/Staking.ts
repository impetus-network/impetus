// ABI for the Staking precompile (IStaking.sol) at 0x0810.
// Nominator-focused subset: bonding lifecycle + nominate + read views.
// Validator-only entrypoints (validate, kick, payoutStakers, ...) are omitted
// here; add them when a full validator-onboarding flow ships.
export const StakingAbi = [
  {
    type: "function",
    name: "bond",
    stateMutability: "nonpayable",
    inputs: [
      { name: "value", type: "uint256" },
      {
        name: "payee",
        type: "tuple",
        components: [
          { name: "kind", type: "uint8" },
          { name: "account", type: "address" },
        ],
      },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "bondExtra",
    stateMutability: "nonpayable",
    inputs: [{ name: "maxAdditional", type: "uint256" }],
    outputs: [],
  },
  {
    type: "function",
    name: "unbond",
    stateMutability: "nonpayable",
    inputs: [{ name: "value", type: "uint256" }],
    outputs: [],
  },
  {
    type: "function",
    name: "withdrawUnbonded",
    stateMutability: "nonpayable",
    inputs: [{ name: "numSlashingSpans", type: "uint32" }],
    outputs: [],
  },
  {
    type: "function",
    name: "rebond",
    stateMutability: "nonpayable",
    inputs: [{ name: "value", type: "uint256" }],
    outputs: [],
  },
  {
    type: "function",
    name: "nominate",
    stateMutability: "nonpayable",
    inputs: [{ name: "targets", type: "address[]" }],
    outputs: [],
  },
  {
    type: "function",
    name: "chill",
    stateMutability: "nonpayable",
    inputs: [],
    outputs: [],
  },
  {
    type: "function",
    name: "validate",
    stateMutability: "nonpayable",
    inputs: [
      {
        name: "prefs",
        type: "tuple",
        components: [
          { name: "commissionPercent", type: "uint16" },
          { name: "blocked", type: "bool" },
        ],
      },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "currentEra",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
  },
  {
    type: "function",
    name: "activeEra",
    stateMutability: "view",
    inputs: [],
    outputs: [
      { name: "index", type: "uint32" },
      { name: "startMs", type: "uint64" },
    ],
  },
  {
    type: "function",
    name: "minNominatorBond",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "minValidatorBond",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "validatorCount",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
  },
  {
    type: "function",
    name: "counterForValidators",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
  },
  {
    type: "function",
    name: "bonded",
    stateMutability: "view",
    inputs: [{ name: "controller", type: "address" }],
    outputs: [{ name: "", type: "address" }],
  },
  {
    type: "function",
    name: "ledger",
    stateMutability: "view",
    inputs: [{ name: "controller", type: "address" }],
    outputs: [
      { name: "active", type: "uint256" },
      { name: "total", type: "uint256" },
      {
        name: "unlocking",
        type: "tuple[]",
        components: [
          { name: "era", type: "uint32" },
          { name: "value", type: "uint256" },
        ],
      },
    ],
  },
  {
    type: "function",
    name: "nominators",
    stateMutability: "view",
    inputs: [{ name: "stash", type: "address" }],
    outputs: [
      { name: "targets", type: "address[]" },
      { name: "submittedIn", type: "uint32" },
      { name: "suppressed", type: "bool" },
    ],
  },
  {
    type: "function",
    name: "validators",
    stateMutability: "view",
    inputs: [{ name: "stash", type: "address" }],
    outputs: [
      { name: "commissionPercent", type: "uint16" },
      { name: "blocked", type: "bool" },
    ],
  },
  {
    type: "function",
    name: "minActiveStake",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "historyDepth",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
  },
  {
    type: "function",
    name: "erasValidatorReward",
    stateMutability: "view",
    inputs: [{ name: "era", type: "uint32" }],
    outputs: [{ name: "", type: "uint256" }],
  },
  {
    type: "function",
    name: "payoutStakers",
    stateMutability: "nonpayable",
    inputs: [
      { name: "validatorStash", type: "address" },
      { name: "era", type: "uint32" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "payoutStakersByPage",
    stateMutability: "nonpayable",
    inputs: [
      { name: "validatorStash", type: "address" },
      { name: "era", type: "uint32" },
      { name: "page", type: "uint32" },
    ],
    outputs: [],
  },
] as const;

// RewardDestination.kind values (see IStaking.sol).
export const REWARD_DESTINATION_STAKED = 0;
