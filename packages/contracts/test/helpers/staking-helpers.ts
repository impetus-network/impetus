import { ethers } from "hardhat";

const STAKING_ADDRESS = "0x0000000000000000000000000000000000000810";
const SESSION_ADDRESS = "0x0000000000000000000000000000000000000818";
const POOLS_ADDRESS = "0x0000000000000000000000000000000000000820";
const FAST_UNSTAKE_ADDRESS = "0x0000000000000000000000000000000000000828";
const TREASURY_ADDRESS = "0x0000000000000000000000000000000000000830";
const BAGS_LIST_ADDRESS = "0x0000000000000000000000000000000000000838";
const STAKING_ADMIN_ADDRESS = "0x0000000000000000000000000000000000000840";

export const ADDRESSES = {
  STAKING: STAKING_ADDRESS,
  SESSION: SESSION_ADDRESS,
  POOLS: POOLS_ADDRESS,
  FAST_UNSTAKE: FAST_UNSTAKE_ADDRESS,
  TREASURY: TREASURY_ADDRESS,
  BAGS_LIST: BAGS_LIST_ADDRESS,
  STAKING_ADMIN: STAKING_ADMIN_ADDRESS,
} as const;

const POLL_INTERVAL_MS = 1_500;
const DEFAULT_TIMEOUT_MS = 5 * 60 * 1_000;

export async function waitForEra(target: number, timeoutMs = DEFAULT_TIMEOUT_MS): Promise<void> {
  const staking = await ethers.getContractAt("IStaking", STAKING_ADDRESS);
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const era = Number(await staking.currentEra());
    if (era >= target) return;
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(`waitForEra(${target}) timed out after ${timeoutMs} ms`);
}

export async function advanceToNextSession(timeoutMs = DEFAULT_TIMEOUT_MS): Promise<number> {
  const session = await ethers.getContractAt("ISession", SESSION_ADDRESS);
  const start = Date.now();
  const initial = Number(await session.currentIndex());
  while (Date.now() - start < timeoutMs) {
    const cur = Number(await session.currentIndex());
    if (cur > initial) return cur;
    await sleep(POLL_INTERVAL_MS);
  }
  throw new Error(`advanceToNextSession timed out after ${timeoutMs} ms`);
}

export async function getStakingLedger(stash: string) {
  const staking = await ethers.getContractAt("IStaking", STAKING_ADDRESS);
  return await staking.ledger(stash);
}

export async function seedDevValidators(): Promise<void> {
  // Genesis already pre-bonds 4 validators in impetus_dev_npos.
  // This helper exists as a no-op for parity with the original spec;
  // future variants may need to inject keys here.
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
