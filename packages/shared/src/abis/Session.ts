// ABI for the Session precompile (ISession.sol) at 0x0818.
// `keys` is the SCALE-encoded session-key bundle (babe, grandpa, im_online,
// authority_discovery = 128 bytes on Impetus) as returned by the node's
// `author_rotateKeys` RPC. `proof` is unused on this runtime — pass "0x".
export const SessionAbi = [
  {
    type: "function",
    name: "setKeys",
    stateMutability: "nonpayable",
    inputs: [
      { name: "keys", type: "bytes" },
      { name: "proof", type: "bytes" },
    ],
    outputs: [],
  },
  {
    type: "function",
    name: "purgeKeys",
    stateMutability: "nonpayable",
    inputs: [],
    outputs: [],
  },
  {
    type: "function",
    name: "currentIndex",
    stateMutability: "view",
    inputs: [],
    outputs: [{ name: "", type: "uint32" }],
  },
  {
    type: "function",
    name: "nextKeys",
    stateMutability: "view",
    inputs: [{ name: "validator", type: "address" }],
    outputs: [{ name: "", type: "bytes" }],
  },
] as const;
