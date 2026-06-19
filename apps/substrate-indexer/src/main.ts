import { TypeormDatabase } from "@subsquid/typeorm-store";
import { MoreThan } from "typeorm";
import { processor, type Call } from "./processor";
import {
  Validator,
  Nominator,
  Era,
  Payout,
  Pool,
  PoolMember,
  StakeEvent,
  Transfer,
  GaslessRule,
  Holder,
  ChainStat,
} from "./model";
import * as m from "./mapping";
import { createEvmWriter, type BalanceRow } from "./sql-writer";
import { EthRpc } from "./evm-rpc";
import { buildEvmRows } from "./evm-build";

// EVM explorer tables: sourced via canonical eth RPC against the SAME archive
// node the processor ingests from (Frontier serves eth_* on the substrate RPC
// endpoint), written to the same Postgres as the Squid store.
const evmWriter = createEvmWriter();
const eth = new EthRpc(process.env.RPC_ENDPOINT as string);

// Extract the signer (0x-hex) of a signed extrinsic. On AccountId20 the call
// origin is `{ __kind: 'system', value: { __kind: 'Signed', value: <address> } }`.
// NOTE: verify this shape against your runtime metadata after typegen.
function signerOf(call: Call): string | undefined {
  const origin = call.origin as
    | { __kind: string; value?: { __kind: string; value?: unknown } }
    | undefined;
  const v = origin?.value;
  if (origin?.__kind === "system" && v?.__kind === "Signed") {
    const addr = v.value;
    if (typeof addr === "string") return addr.toLowerCase();
    if (addr instanceof Uint8Array) return "0x" + Buffer.from(addr).toString("hex");
  }
  return undefined;
}

processor.run(new TypeormDatabase({ supportHotBlocks: true }), async (ctx) => {
  await evmWriter.init();

  const validators = new Map<string, Validator>();
  const nominators = new Map<string, Nominator>();
  const pools = new Map<number, Pool>();
  const members = new Map<string, PoolMember>();
  const payouts = new Map<string, Payout>();
  const eras = new Map<string, Era>();
  const stakeEvents: StakeEvent[] = [];
  const transfers: Transfer[] = [];
  const gaslessRules = new Map<string, GaslessRule>();
  // Native accounts whose balance changed this batch; their authoritative
  // balance is read from System.Account storage after the block loop.
  const touchedAccounts = new Set<string>();

  async function getValidator(id: string): Promise<Validator> {
    let v = validators.get(id) ?? (await ctx.store.get(Validator, id));
    if (!v) {
      v = new Validator({
        id,
        commission: 0,
        blocked: false,
        active: false,
        elected: false,
        selfBonded: 0n,
        blocksProduced: 0,
        lastBlock: null,
        updatedAt: 0,
      });
    }
    validators.set(id, v);
    return v;
  }
  async function getPool(id: number, height: number, creator = "0x"): Promise<Pool> {
    let p = pools.get(id) ?? (await ctx.store.get(Pool, String(id)));
    if (!p) {
      p = new Pool({ id: String(id), creator, state: "Open", createdBlock: height, updatedAt: height });
    }
    pools.set(id, p);
    return p;
  }
  async function getMember(id: string, height: number): Promise<PoolMember> {
    let mem = members.get(id) ?? (await ctx.store.get(PoolMember, id));
    if (!mem) {
      mem = new PoolMember({ id, poolId: 0, bonded: 0n, lastClaimBlock: null, updatedAt: height });
    }
    members.set(id, mem);
    return mem;
  }

  for (const block of ctx.blocks) {
    const height = block.header.height;
    const timestamp = new Date(block.header.timestamp ?? 0);

    // Block authorship: the producing validator (AccountId20 = 0x-hex). Real
    // activity even on an idle chain — counts blocks per validator.
    const author = block.header.validator;
    if (author) {
      const v = await getValidator(author.toLowerCase());
      v.blocksProduced += 1;
      v.lastBlock = height;
      v.updatedAt = height;
    }

    for (const event of block.events) {
      // Flag accounts touched by any balance event (Transfer/fees/rewards/etc.)
      // so we re-read their authoritative balance from storage below.
      for (const a of m.balanceTouchedAccounts(event)) touchedAccounts.add(a);

      switch (event.name) {
        case "Staking.ValidatorPrefsSet": {
          const r = m.decodeValidatorPrefsSet(event);
          const v = await getValidator(r.account);
          v.commission = r.commissionPercent;
          v.blocked = r.blocked;
          v.active = true;
          v.updatedAt = height;
          break;
        }
        case "Staking.Chilled": {
          const r = m.decodeChilled(event);
          const v = await getValidator(r.account);
          v.active = false;
          v.updatedAt = height;
          const n = nominators.get(r.account) ?? (await ctx.store.get(Nominator, r.account));
          if (n) {
            n.active = false;
            n.updatedAt = height;
            nominators.set(r.account, n);
          }
          break;
        }
        case "Staking.Bonded": {
          const r = m.decodeBonded(event);
          (await getValidator(r.account)).updatedAt = height; // touch; exact bond via storage
          stakeEvents.push(
            new StakeEvent({ id: event.id, account: r.account, kind: "Bonded", amount: r.amount, block: height, timestamp }),
          );
          break;
        }
        case "Staking.Unbonded": {
          const r = m.decodeUnbonded(event);
          stakeEvents.push(
            new StakeEvent({ id: event.id, account: r.account, kind: "Unbonded", amount: r.amount, block: height, timestamp }),
          );
          break;
        }
        case "Staking.Withdrawn": {
          const r = m.decodeWithdrawn(event);
          stakeEvents.push(
            new StakeEvent({ id: event.id, account: r.account, kind: "Withdrawn", amount: r.amount, block: height, timestamp }),
          );
          break;
        }
        case "Staking.Rewarded": {
          const r = m.decodeRewarded(event);
          stakeEvents.push(
            new StakeEvent({ id: event.id, account: r.account, kind: "Rewarded", amount: r.amount, block: height, timestamp }),
          );
          break;
        }
        case "Staking.Slashed": {
          const r = m.decodeSlashed(event);
          stakeEvents.push(
            new StakeEvent({ id: event.id, account: r.account, kind: "Slashed", amount: r.amount, block: height, timestamp }),
          );
          break;
        }
        case "Staking.PayoutStarted": {
          const r = m.decodePayoutStarted(event);
          const id = `${r.validator}-${r.era}`;
          payouts.set(id, new Payout({ id, validator: r.validator, era: r.era, block: height }));
          break;
        }
        case "Staking.EraPaid": {
          const r = m.decodeEraPaid(event);
          eras.set(String(r.era), new Era({ id: String(r.era), index: r.era, validatorReward: r.validatorPayout, startBlock: height }));
          break;
        }
        case "Staking.StakersElected": {
          // TODO: read Session.Validators storage at this block and set
          // Validator.elected = true for the active set (false for the rest).
          // Requires the generated storage decoder (typegen.json Session.Validators).
          break;
        }
        case "NominationPools.Created": {
          const r = m.decodePoolCreated(event);
          const p = await getPool(r.poolId, height, r.creator);
          p.creator = r.creator;
          p.updatedAt = height;
          break;
        }
        case "NominationPools.Bonded": {
          const r = m.decodePoolBonded(event);
          const mem = await getMember(r.member, height);
          mem.poolId = r.poolId;
          mem.bonded += r.bonded;
          mem.updatedAt = height;
          break;
        }
        case "NominationPools.PaidOut": {
          const r = m.decodePoolPaidOut(event);
          const mem = await getMember(r.member, height);
          mem.poolId = r.poolId;
          mem.lastClaimBlock = height;
          mem.updatedAt = height;
          break;
        }
        case "NominationPools.Unbonded": {
          const r = m.decodePoolUnbonded(event);
          const mem = await getMember(r.member, height);
          mem.poolId = r.poolId;
          mem.updatedAt = height;
          break;
        }
        case "NominationPools.StateChanged": {
          const r = m.decodePoolStateChanged(event);
          const p = await getPool(r.poolId, height);
          p.state = r.state;
          p.updatedAt = height;
          break;
        }
        case "Balances.Transfer": {
          // Every native value move: EOA->EOA, EOA->contract, and internal
          // contract->contract (Frontier funnels EVM value through pallet_balances).
          // Tx fees use Balances.Withdraw/Deposit (not Transfer), so this stays
          // close to real transfers; filter system addresses downstream if needed.
          const r = m.decodeTransfer(event);
          transfers.push(
            new Transfer({
              id: event.id,
              from: r.from,
              to: r.to,
              amount: r.amount,
              block: height,
              timestamp,
              extrinsicHash: event.extrinsic?.hash,
            }),
          );
          break;
        }
        case "GaslessRegistry.RuleSet": {
          const r = m.decodeRuleSet(event);
          const id = `${r.contract}-${r.selector}`;
          gaslessRules.set(
            id,
            new GaslessRule({
              id,
              contract: r.contract,
              selector: r.selector,
              enabled: r.enabled,
              minValue: r.minValue,
              updatedAtBlock: height,
            }),
          );
          break;
        }
        case "GaslessRegistry.RuleRemoved": {
          const r = m.decodeRuleRemoved(event);
          const id = `${r.contract}-${r.selector}`;
          gaslessRules.set(
            id,
            new GaslessRule({
              id,
              contract: r.contract,
              selector: r.selector,
              enabled: false,
              minValue: 0n,
              updatedAtBlock: height,
            }),
          );
          break;
        }
        default:
          break;
      }
    }

    for (const call of block.calls) {
      if (call.name === "Staking.nominate" && call.success) {
        const account = signerOf(call);
        if (!account) continue;
        const { targets } = m.decodeNominateCall(call);
        let n = nominators.get(account) ?? (await ctx.store.get(Nominator, account));
        if (!n) n = new Nominator({ id: account, targets, active: true, updatedAt: height });
        n.targets = targets;
        n.active = true;
        n.updatedAt = height;
        nominators.set(account, n);
      }
    }
  }

  // Pools before members (FK-free, but keeps logical order). Order otherwise free.
  await ctx.store.upsert([...pools.values()]);
  await ctx.store.upsert([...validators.values()]);
  await ctx.store.upsert([...nominators.values()]);
  await ctx.store.upsert([...members.values()]);
  await ctx.store.upsert([...eras.values()]);
  await ctx.store.upsert([...payouts.values()]);
  await ctx.store.insert(stakeEvents);
  await ctx.store.insert(transfers);
  await ctx.store.upsert([...gaslessRules.values()]);

  // ---- EVM explorer tables (evm_blocks / evm_transactions / evm_contracts) --
  // Source canonical EVM data for every block in the batch via eth RPC, then
  // write idempotently (DELETE-range + INSERT) so reorgs self-heal on Squid's
  // re-delivery from the fork point.
  if (ctx.blocks.length > 0) {
    const heights = ctx.blocks.map((b) => b.header.height);
    const firstHeight = heights[0]!;
    const evmRows = await buildEvmRows(eth, heights);
    await evmWriter.writeBatch(firstHeight, evmRows);
  }

  // ---- Native holders (Pattern 1) ------------------------------------------
  // Read authoritative balances from System.Account storage at the batch head:
  // a one-time full seed scan (so genesis accounts appear even when resuming at
  // the chain head), then an incremental refresh of only the touched accounts.
  if (ctx.blocks.length > 0) {
    const head = ctx.blocks[ctx.blocks.length - 1].header;
    const holders = new Map<string, Holder>();
    // Same authoritative balances feed the explorer's balance_accounts table
    // (balance<-free, locked<-frozen, reserved<-reserved, nonce).
    const accountBalances = new Map<string, m.AccountBalance>();

    const toHolder = (address: string, bal: m.AccountBalance): Holder =>
      new Holder({
        id: address,
        free: bal.free,
        reserved: bal.reserved,
        total: bal.free + bal.reserved,
        nonce: bal.nonce,
        updatedAt: head.height,
      });

    let stat =
      (await ctx.store.get(ChainStat, "singleton")) ??
      new ChainStat({
        id: "singleton",
        totalIssuance: 0n,
        holdersCount: 0,
        seeded: false,
        updatedAt: 0,
      });

    if (!stat.seeded) {
      for await (const [address, bal] of m.scanAllAccounts(head)) {
        holders.set(address, toHolder(address, bal));
        accountBalances.set(address, bal);
      }
      stat.seeded = true;
    }

    if (touchedAccounts.size > 0) {
      const balances = await m.readAccounts(head, [...touchedAccounts]);
      for (const [address, bal] of balances) {
        holders.set(address, toHolder(address, bal));
        accountBalances.set(address, bal);
      }
    }

    if (holders.size > 0) await ctx.store.upsert([...holders.values()]);

    if (accountBalances.size > 0) {
      const balanceRows: BalanceRow[] = [];
      for (const [address, bal] of accountBalances) {
        balanceRows.push({
          address,
          balance: bal.free.toString(),
          locked: bal.frozen.toString(),
          reserved: bal.reserved.toString(),
          nonce: String(bal.nonce),
        });
      }
      await evmWriter.upsertBalances(balanceRows);
    }

    stat.totalIssuance = await m.readTotalIssuance(head);
    stat.holdersCount = await ctx.store.countBy(Holder, { total: MoreThan(0n) });
    stat.updatedAt = head.height;
    await ctx.store.upsert(stat);
  }
});
