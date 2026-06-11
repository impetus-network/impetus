import type {
  DemoToken,
  MockBlock,
  MockTransaction,
  TxKind,
  ValidatorRow,
} from "./types";

const txTypes: TxKind[] = [
  "transfer",
  "swap",
  "mint",
  "contract",
  "transfer",
  "transfer",
];

export const demoTokens: DemoToken[] = [
  { sym: "ART", name: "Artemis", balance: 1248.42, usd: 1.84, color: "#ffb084" },
  { sym: "USDC", name: "USD Coin", balance: 5430, usd: 1, color: "#2775ca" },
  { sym: "USDT", name: "Tether", balance: 200.5, usd: 1, color: "#26a17b" },
  { sym: "WETH", name: "Wrapped ETH", balance: 2.4831, usd: 3210, color: "#627eea" },
  { sym: "WBTC", name: "Wrapped BTC", balance: 0.1422, usd: 68420, color: "#f7931a" },
  { sym: "AGT", name: "Argent", balance: 89000, usd: 0.012, color: "#b8a4ed" },
];

export const validators: ValidatorRow[] = [
  { rank: 1, name: "Selene Labs", stake: "4.82M", blocks: "218,492", uptime: "100.00%" },
  { rank: 2, name: "Phoebe Stake", stake: "3.91M", blocks: "177,210", uptime: "99.99%" },
  { rank: 3, name: "Nyx Validators", stake: "3.44M", blocks: "156,003", uptime: "99.98%" },
  { rank: 4, name: "Hecate Node", stake: "2.87M", blocks: "129,477", uptime: "100.00%" },
  { rank: 5, name: "Cynthia Capital", stake: "2.51M", blocks: "113,290", uptime: "99.97%" },
];

function mixSeed(value: number): number {
  const firstPass = Math.imul(value ^ (value >>> 16), 0x7feb352d);
  const secondPass = Math.imul(firstPass ^ (firstPass >>> 15), 0x846ca68b);

  return (secondPass ^ (secondPass >>> 16)) >>> 0;
}

function hexFromSeed(seed: number, length: number): string {
  return Array.from({ length }, (_, index) => {
    const mixed = mixSeed((seed + Math.imul(index + 1, 0x9e3779b9)) >>> 0);

    return (mixed & 0xf).toString(16);
  }).join("");
}

export function makeAddress(seed: number): `0x${string}` {
  return `0x${hexFromSeed(seed, 40)}`;
}

export function makeHash(seed: number): `0x${string}` {
  return `0x${hexFromSeed(seed + 1000, 64)}`;
}

export function shortHash(value: string): string {
  return `${value.slice(0, 6)}...${value.slice(-4)}`;
}

export function makeTransaction(seed: number, age: number): MockTransaction {
  const type = txTypes[seed % txTypes.length];
  const value = ((seed * 37.119 + 12.48) % 500).toFixed(3);

  return {
    hash: makeHash(seed),
    from: makeAddress(seed + 20),
    to: makeAddress(seed + 40),
    value,
    type,
    age,
  };
}

export function seedTransactions(count: number): MockTransaction[] {
  return Array.from({ length: count }, (_, index) =>
    makeTransaction(index + 1, index * 2),
  );
}

export function makeBlocks(latestBlock: number): MockBlock[] {
  return Array.from({ length: 8 }, (_, index) => ({
    number: latestBlock - index,
    hash: makeHash(latestBlock - index),
    txCount: 50 + ((latestBlock + index * 29) % 250),
    proposer: `${makeAddress(latestBlock + index).slice(0, 10)}...`,
    age: index * 0.4,
  }));
}

export function formatUsd(value: number): string {
  return value.toLocaleString("en-US", { maximumFractionDigits: 0 });
}
