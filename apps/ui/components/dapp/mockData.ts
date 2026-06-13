// The block explorer and live feed read real chain data via RPC; the only
// shared helper kept here is address/hash shortening. Former demo data
// (demoTokens, validators, seedTransactions, makeBlocks, ...) was unused and
// removed — surfaces now show real on-chain values.
export function shortHash(value: string): string {
  return `${value.slice(0, 6)}...${value.slice(-4)}`;
}
