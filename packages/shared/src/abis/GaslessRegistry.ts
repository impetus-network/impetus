export const GaslessRegistryAbi = [
  {
    type: "event",
    name: "RuleSet",
    inputs: [
      { name: "contract_", type: "address", indexed: true },
      { name: "selector", type: "bytes4", indexed: true },
      { name: "enabled", type: "bool", indexed: false },
      { name: "minValue", type: "uint256", indexed: false },
    ],
  },
  {
    type: "event",
    name: "RuleRemoved",
    inputs: [
      { name: "contract_", type: "address", indexed: true },
      { name: "selector", type: "bytes4", indexed: true },
    ],
  },
  {
    type: "function",
    name: "getRule",
    stateMutability: "view",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "selector", type: "bytes4" },
    ],
    outputs: [
      { name: "enabled", type: "bool" },
      { name: "minValue", type: "uint256" },
    ],
  },
  {
    type: "function",
    name: "isGasless",
    stateMutability: "view",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "input", type: "bytes" },
      { name: "value", type: "uint256" },
      { name: "gasLimit", type: "uint256" },
    ],
    outputs: [{ name: "", type: "bool" }],
  },
  {
    type: "function",
    name: "setRule",
    stateMutability: "nonpayable",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "selector", type: "bytes4" },
      { name: "minValue", type: "uint256" },
      { name: "enabled", type: "bool" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "removeRule",
    stateMutability: "nonpayable",
    inputs: [
      { name: "contract_", type: "address" },
      { name: "selector", type: "bytes4" },
    ],
    outputs: [],
  },
] as const;
