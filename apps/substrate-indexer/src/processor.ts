import {
  SubstrateBatchProcessor,
  type SubstrateBatchProcessorFields,
  type BlockHeader,
  type Event as _Event,
  type Call as _Call,
} from "@subsquid/substrate-processor";

const RPC_ENDPOINT = process.env.RPC_ENDPOINT;
if (!RPC_ENDPOINT) {
  throw new Error("RPC_ENDPOINT is required (Substrate RPC of an Impetus archive node)");
}

// Staking + nomination-pools signals. pallet-staking has NO `Nominated` event,
// so nominators are tracked via the `Staking.nominate` call below.
const STAKING_EVENTS = [
  "Staking.Bonded",
  "Staking.Unbonded",
  "Staking.Withdrawn",
  "Staking.ValidatorPrefsSet",
  "Staking.Chilled",
  "Staking.Kicked",
  "Staking.PayoutStarted",
  "Staking.Rewarded",
  "Staking.Slashed",
  "Staking.StakersElected",
  "Staking.EraPaid",
];

const POOL_EVENTS = [
  "NominationPools.Created",
  "NominationPools.Bonded",
  "NominationPools.PaidOut",
  "NominationPools.Unbonded",
  "NominationPools.Withdrawn",
  "NominationPools.StateChanged",
  "NominationPools.MemberRemoved",
  "NominationPools.PoolCommissionUpdated",
];

// Native transfers: Frontier routes every EVM value move (top-level AND
// internal contract->contract) through pallet_balances, so a single
// Balances.Transfer event captures them all — no trace needed.
//
// The non-Transfer events are subscribed only to flag WHICH accounts changed
// balance in a block (fees, rewards, slashing, reserves) so the holder index
// can re-read System.Account for them — the authoritative amount comes from
// storage, not from these event payloads.
const BALANCES_EVENTS = [
  "Balances.Transfer",
  "Balances.Endowed",
  "Balances.Deposit",
  "Balances.Withdraw",
  "Balances.Reserved",
  "Balances.Unreserved",
  "Balances.Slashed",
  "Balances.BalanceSet",
  "Balances.Minted",
  "Balances.Burned",
];

// Gasless registry rules (substrate events from pallet-gasless-registry).
const GASLESS_EVENTS = ["GaslessRegistry.RuleSet", "GaslessRegistry.RuleRemoved"];

// RPC-only ingestion: Impetus is not an SQD Portal dataset, so we do NOT call
// .setGateway(). Point at an archive node; backfill from 0 needs full history.
export const processor = new SubstrateBatchProcessor()
  .setRpcEndpoint({ url: RPC_ENDPOINT, rateLimit: 10 })
  .setBlockRange({ from: 0 })
  .addEvent({
    name: [...STAKING_EVENTS, ...POOL_EVENTS, ...BALANCES_EVENTS, ...GASLESS_EVENTS],
    extrinsic: true,
  })
  .addCall({ name: ["Staking.nominate"], extrinsic: true })
  // Need EVERY block (not just ones with matching events) to count block
  // authorship per validator. `block.validator` is the author's account.
  .includeAllBlocks()
  .setFields({
    block: { timestamp: true, validator: true },
    event: { args: true },
    call: { args: true, origin: true, success: true },
    extrinsic: { hash: true },
  });

export type Fields = SubstrateBatchProcessorFields<typeof processor>;
export type Block = BlockHeader<Fields>;
export type Event = _Event<Fields>;
export type Call = _Call<Fields>;
