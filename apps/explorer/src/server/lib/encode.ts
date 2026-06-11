import { getAddress } from "viem";
import type { Decimal } from "@prisma/client/runtime/library";

/** Checksum an address string (already 0x-prefixed varchar from DB). */
export function checksumAddr(s: string): `0x${string}` {
  return getAddress(s as `0x${string}`);
}

/** Convert Prisma bigint to safe JS number (for block_num, nonce, etc.). */
export function safeNum(n: bigint): number {
  if (n > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error(`safeNum overflow: ${n}`);
  }
  return Number(n);
}

/** Convert Prisma Decimal to string (for value, gas, balance). */
export function decStr(d: Decimal): string {
  return d.toFixed(0);
}

/** Apply fn to non-null value, return null for null. */
export function nullable<T, U>(x: T | null, fn: (t: T) => U): U | null {
  return x === null ? null : fn(x);
}
