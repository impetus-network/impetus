export type TxKind = "transfer" | "swap" | "mint" | "contract";

export type MockTransaction = {
  hash: `0x${string}`;
  from: `0x${string}`;
  to: `0x${string}`;
  value: string;
  type: TxKind;
  age: number;
};

export type LiveFeedState = {
  tps: number;
  block: number;
  gasPrice: 0;
  txs: MockTransaction[];
};

export type DemoToken = {
  sym: string;
  name: string;
  balance: number;
  usd: number;
  color: string;
};

export type MockBlock = {
  number: number;
  hash: `0x${string}`;
  txCount: number;
  proposer: string;
  age: number;
};

export type ValidatorRow = {
  rank: number;
  name: string;
  stake: string;
  blocks: string;
  uptime: string;
};
