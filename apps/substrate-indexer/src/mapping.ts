// Decoding glue between the generated typegen modules and the handler logic.
//
// THIS IS THE ONLY FILE THAT TOUCHES GENERATED TYPES. After running
// `npm run metadata && npm run typegen`, verify:
//   1. The import paths below match the generated file names in src/types/
//      (pallet -> snake_case file, e.g. Staking -> staking.ts).
//   2. The version key `.v{N}` matches what typegen emitted for each item
//      (it is keyed by the spec_version where the type was introduced; for a
//      single-runtime chain there is usually one key). Add more versions to the
//      `pick([...])` lists if typegen generates several.
//
// AccountId on Impetus is AccountId20 (= H160), so accounts decode to 0x-hex
// strings directly — no SS58 conversion needed.

import * as stakingEvents from "./types/staking/events";
import * as stakingCalls from "./types/staking/calls";
import * as poolEvents from "./types/nomination-pools/events";
import * as balancesEvents from "./types/balances/events";
import * as gaslessEvents from "./types/gasless-registry/events";
import * as systemStorage from "./types/system/storage";
import * as balancesStorage from "./types/balances/storage";
import type { Event, Call, Block } from "./processor";

// Pick the first runtime version whose `.is(item)` matches. Lets one handler
// cover multiple runtime versions without branching at the call site.
interface Versioned<I, T> {
  is(item: I): boolean;
  decode(item: I): T;
}
function pickEvent<T>(versions: Versioned<Event, T>[], e: Event): T {
  for (const v of versions) if (v.is(e)) return v.decode(e);
  throw new Error(`no matching runtime version for event ${e.name}`);
}
function pickCall<T>(versions: Versioned<Call, T>[], c: Call): T {
  for (const v of versions) if (v.is(c)) return v.decode(c);
  throw new Error(`no matching runtime version for call ${c.name}`);
}

function toAddress(value: unknown): string {
  if (typeof value === "string") return value.toLowerCase();
  if (value instanceof Uint8Array) {
    return "0x" + Buffer.from(value).toString("hex");
  }
  throw new Error("unexpected account encoding");
}

// Generic bytes -> 0x-hex (e.g. a 4-byte selector). Same shape as toAddress.
function toHex(value: unknown): string {
  return toAddress(value);
}

// ---- Staking ---------------------------------------------------------------

export interface AccountAmount {
  account: string;
  amount: bigint;
}

export function decodeBonded(e: Event): AccountAmount {
  const { stash, amount } = pickEvent([stakingEvents.bonded.v9], e);
  return { account: toAddress(stash), amount };
}
export function decodeUnbonded(e: Event): AccountAmount {
  const { stash, amount } = pickEvent([stakingEvents.unbonded.v9], e);
  return { account: toAddress(stash), amount };
}
export function decodeWithdrawn(e: Event): AccountAmount {
  const { stash, amount } = pickEvent([stakingEvents.withdrawn.v9], e);
  return { account: toAddress(stash), amount };
}
export function decodeRewarded(e: Event): AccountAmount {
  const { stash, amount } = pickEvent([stakingEvents.rewarded.v9], e);
  return { account: toAddress(stash), amount };
}
export function decodeSlashed(e: Event): AccountAmount {
  const { staker, amount } = pickEvent([stakingEvents.slashed.v9], e);
  return { account: toAddress(staker), amount };
}

export function decodeValidatorPrefsSet(e: Event): {
  account: string;
  commissionPercent: number;
  blocked: boolean;
} {
  const { stash, prefs } = pickEvent([stakingEvents.validatorPrefsSet.v9], e);
  // Perbill (0..1e9) -> percent.
  return {
    account: toAddress(stash),
    commissionPercent: Math.round(Number(prefs.commission) / 10_000_000),
    blocked: prefs.blocked,
  };
}

export function decodeChilled(e: Event): { account: string } {
  const { stash } = pickEvent([stakingEvents.chilled.v9], e);
  return { account: toAddress(stash) };
}

export function decodePayoutStarted(e: Event): { validator: string; era: number } {
  const { eraIndex, validatorStash } = pickEvent([stakingEvents.payoutStarted.v9], e);
  return { validator: toAddress(validatorStash), era: eraIndex };
}

export function decodeEraPaid(e: Event): { era: number; validatorPayout: bigint } {
  const { eraIndex, validatorPayout } = pickEvent([stakingEvents.eraPaid.v9], e);
  return { era: eraIndex, validatorPayout };
}

export function decodeNominateCall(c: Call): { targets: string[] } {
  const { targets } = pickCall([stakingCalls.nominate.v9], c);
  // targets are MultiAddress; on AccountId20 they decode to { __kind:'Id', value }.
  const ids = (targets as { value?: unknown }[]).map((t) =>
    toAddress((t as { value?: unknown }).value ?? t),
  );
  return { targets: ids };
}

// ---- Nomination pools ------------------------------------------------------

export function decodePoolCreated(e: Event): { creator: string; poolId: number } {
  const { depositor, poolId } = pickEvent([poolEvents.created.v9], e);
  return { creator: toAddress(depositor), poolId };
}
export function decodePoolBonded(e: Event): {
  member: string;
  poolId: number;
  bonded: bigint;
} {
  const { member, poolId, bonded } = pickEvent([poolEvents.bonded.v9], e);
  return { member: toAddress(member), poolId, bonded };
}
export function decodePoolPaidOut(e: Event): { member: string; poolId: number } {
  const { member, poolId } = pickEvent([poolEvents.paidOut.v9], e);
  return { member: toAddress(member), poolId };
}
export function decodePoolUnbonded(e: Event): { member: string; poolId: number } {
  const { member, poolId } = pickEvent([poolEvents.unbonded.v9], e);
  return { member: toAddress(member), poolId };
}
export function decodePoolStateChanged(e: Event): { poolId: number; state: string } {
  const { poolId, newState } = pickEvent([poolEvents.stateChanged.v9], e);
  return { poolId, state: (newState as { __kind: string }).__kind };
}

// ---- Native transfers ------------------------------------------------------

export function decodeTransfer(e: Event): { from: string; to: string; amount: bigint } {
  const { from, to, amount } = pickEvent([balancesEvents.transfer.v9], e);
  return { from: toAddress(from), to: toAddress(to), amount };
}

// ---- Gasless registry ------------------------------------------------------

export function decodeRuleSet(e: Event): {
  contract: string;
  selector: string;
  enabled: boolean;
  minValue: bigint;
} {
  const { contract, selector, enabled, minValue } = pickEvent([gaslessEvents.ruleSet.v9], e);
  return { contract: toAddress(contract), selector: toHex(selector), enabled, minValue };
}

export function decodeRuleRemoved(e: Event): { contract: string; selector: string } {
  const { contract, selector } = pickEvent([gaslessEvents.ruleRemoved.v9], e);
  return { contract: toAddress(contract), selector: toHex(selector) };
}

// ---- Native holders (Pattern 1: events flag who changed, storage holds truth) -

// Accounts whose native balance changed in this event. Covers every
// balance-mutating Balances event so no holder drifts; returns [] for anything
// else. We only need the addresses — the authoritative amount comes from storage.
export function balanceTouchedAccounts(e: Event): string[] {
  switch (e.name) {
    case "Balances.Transfer": {
      const { from, to } = balancesEvents.transfer.v9.decode(e);
      return [toAddress(from), toAddress(to)];
    }
    case "Balances.Endowed":
      return [toAddress(balancesEvents.endowed.v9.decode(e).account)];
    case "Balances.BalanceSet":
      return [toAddress(balancesEvents.balanceSet.v9.decode(e).who)];
    case "Balances.Reserved":
      return [toAddress(balancesEvents.reserved.v9.decode(e).who)];
    case "Balances.Unreserved":
      return [toAddress(balancesEvents.unreserved.v9.decode(e).who)];
    case "Balances.Deposit":
      return [toAddress(balancesEvents.deposit.v9.decode(e).who)];
    case "Balances.Withdraw":
      return [toAddress(balancesEvents.withdraw.v9.decode(e).who)];
    case "Balances.Slashed":
      return [toAddress(balancesEvents.slashed.v9.decode(e).who)];
    case "Balances.Minted":
      return [toAddress(balancesEvents.minted.v9.decode(e).who)];
    case "Balances.Burned":
      return [toAddress(balancesEvents.burned.v9.decode(e).who)];
    default:
      return [];
  }
}

export interface AccountBalance {
  free: bigint;
  reserved: bigint;
  frozen: bigint; // locks on free balance — explorer `balance_accounts.locked`
  nonce: number;
}

// Authoritative System.Account read for a set of addresses at a block.
// Undefined entries (reaped/never-existed accounts) map to a zero balance.
export async function readAccounts(
  block: Block,
  addresses: string[],
): Promise<Map<string, AccountBalance>> {
  const out = new Map<string, AccountBalance>();
  if (addresses.length === 0) return out;
  const infos = await systemStorage.account.v9.getMany(block, addresses);
  addresses.forEach((addr, i) => {
    const info = infos[i];
    out.set(
      addr,
      info
        ? {
            free: info.data.free,
            reserved: info.data.reserved,
            frozen: info.data.frozen,
            nonce: info.nonce,
          }
        : { free: 0n, reserved: 0n, frozen: 0n, nonce: 0 },
    );
  });
  return out;
}

export async function readTotalIssuance(block: Block): Promise<bigint> {
  return (await balancesStorage.totalIssuance.v9.get(block)) ?? 0n;
}

// One-time seed: stream the entire System.Account map at `block` so
// genesis-funded accounts (which emit no events) appear immediately.
export async function* scanAllAccounts(
  block: Block,
): AsyncIterable<[string, AccountBalance]> {
  for await (const pairs of systemStorage.account.v9.getPairsPaged(500, block)) {
    for (const [addr, info] of pairs) {
      if (!info) continue;
      yield [
        toAddress(addr),
        {
          free: info.data.free,
          reserved: info.data.reserved,
          frozen: info.data.frozen,
          nonce: info.nonce,
        },
      ];
    }
  }
}
