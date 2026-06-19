"use client";

import { useMemo, useState, type FormEvent, type ReactElement } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { formatEther } from "viem";
import {
  DappPanel,
  Mono,
  OverviewCard,
  TransactionTypeBadge,
} from "./DappPrimitives";
import { shortHash } from "./mockData";
import { HoldersPanel } from "./HoldersPanel";
import { useFetchBlocks } from "~/hooks/useFetchBlocks";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { resolveSearch } from "~/components/blockexplorer/SearchBar";
import type { LiveFeedState, TxKind } from "./types";

const filters: Array<TxKind | "all"> = [
  "all",
  "transfer",
  "swap",
  "mint",
  "contract",
];

const searchHelperId = "explorer-search-helper";

function classifyTx(to: string | null): TxKind {
  if (!to) return "contract";
  const last = parseInt(to.slice(-1), 16);
  if (last < 2) return "swap";
  if (last < 4) return "mint";
  if (last < 6) return "contract";
  return "transfer";
}

function timeAgo(timestamp: bigint): string {
  const now = Math.floor(Date.now() / 1000);
  const diff = now - Number(timestamp);
  if (diff < 2) return "just now";
  if (diff < 60) return `${diff}s ago`;
  return `${Math.floor(diff / 60)}m ago`;
}

export function ExplorerPanels({
  feed,
}: {
  feed: LiveFeedState;
}): ReactElement {
  const router = useRouter();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<TxKind | "all">("all");
  const [view, setView] = useState<"activity" | "holders">("activity");

  function onSearch(event: FormEvent) {
    event.preventDefault();
    const target = resolveSearch(search);
    if (target) {
      router.push(target);
      setSearch("");
    }
  }
  const { blocks: realBlocks, transactions: realTxs } = useFetchBlocks(8);
  const { data: validatorCount } = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "counterForValidators",
  });

  const explorerTxs = useMemo(() => {
    const mapped = realTxs.map((tx) => ({
      hash: tx.hash,
      from: tx.from as `0x${string}`,
      to: (tx.to ?? tx.from) as `0x${string}`,
      value: Number(formatEther(tx.value)).toFixed(3),
      type: classifyTx(tx.to ?? null),
      age: 0,
    }));
    if (filter === "all") return mapped;
    return mapped.filter((tx) => tx.type === filter);
  }, [realTxs, filter]);

  return (
    <div className="mt-10 space-y-6">
      <form
        className="flex items-center gap-2 rounded-3xl bg-[#f5f0e0] p-2"
        onSubmit={onSearch}
      >
        <div className="flex min-w-0 flex-1 items-center gap-3">
          <span className="pl-4 text-lg text-[#6a6a6a]">⌕</span>
          <label className="sr-only" htmlFor="explorer-search">
            Search explorer
          </label>
          <input
            aria-describedby={searchHelperId}
            className="min-h-12 min-w-0 flex-1 bg-transparent font-mono text-sm text-[#0a0a0a] outline-none placeholder:text-[#6a6a6a]"
            id="explorer-search"
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search by address, tx hash, block number"
            type="search"
            value={search}
          />
        </div>
        <button
          aria-describedby={searchHelperId}
          className="inline-flex min-h-12 items-center justify-center rounded-md bg-[#0a0a0a] px-6 text-sm font-semibold text-white transition hover:bg-[#1f1f1f]"
          type="submit"
        >
          Search
        </button>
      </form>

      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <OverviewCard
          label="Latest block"
          tone="lavender"
          value={<Mono>#{feed.block.toLocaleString()}</Mono>}
        />
        <OverviewCard
          label="TPS"
          sub={feed.tps > 0 ? "↑ trending up" : "awaiting blocks"}
          tone="ochre"
          value={<Mono>{feed.tps.toLocaleString()}</Mono>}
        />
        <OverviewCard
          label="Validators"
          sub="active set"
          tone="peach"
          value={
            <Mono>
              {validatorCount !== undefined ? Number(validatorCount).toLocaleString() : "—"}
            </Mono>
          }
        />
        <OverviewCard
          label="Avg gas paid"
          sub="gasless network"
          tone="cream"
          value={<Mono>$0.00</Mono>}
        />
      </section>

      <div className="flex w-fit gap-1 rounded-full bg-[#f5f0e0] p-1">
        {(["activity", "holders"] as const).map((item) => (
          <button
            className="rounded-full px-4 py-1.5 text-[12px] font-semibold capitalize transition data-[active=true]:bg-[#0a0a0a] data-[active=true]:text-white"
            data-active={view === item}
            key={item}
            onClick={() => setView(item)}
            type="button"
          >
            {item}
          </button>
        ))}
      </div>

      {view === "holders" ? (
        <HoldersPanel />
      ) : (
      <div className="grid gap-6 xl:grid-cols-[minmax(0,0.96fr)_minmax(0,1.04fr)]">
        <DappPanel className="overflow-hidden">
          <div className="flex items-center justify-between border-b border-[#0a0a0a]/10 px-5 py-4">
            <span className="text-base font-semibold">Latest blocks</span>
            <Link
              className="text-xs text-[#6a6a6a] hover:text-[#0a0a0a]"
              href="/blockexplorer/blocks"
            >
              View all blocks →
            </Link>
          </div>
          <div className="divide-y divide-[#0a0a0a]/10">
            {realBlocks.length === 0 && (
              <p className="px-5 py-4 text-sm text-[#6a6a6a]">
                Loading blocks...
              </p>
            )}
            {realBlocks.map((block) => (
              <Link
                className="grid grid-cols-[auto_1fr_auto] items-center gap-3 px-4 py-3.5 transition hover:bg-[#f5f0e0]"
                href={`/blockexplorer/block/${Number(block.number)}`}
                key={block.hash}
              >
                <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-[#f5f0e0] font-mono text-[11px] text-[#6a6a6a]">
                  BLK
                </span>
                <div className="min-w-0">
                  <p className="text-[13px] font-semibold">
                    <Mono>#{Number(block.number).toLocaleString()}</Mono>
                  </p>
                  <p className="mt-0.5 text-[11px] text-[#6a6a6a]">
                    by{" "}
                    <Mono>
                      {block.miner
                        ? shortHash(block.miner)
                        : "unknown"}
                    </Mono>
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-[13px] font-medium">
                    {block.transactions.length} txs
                  </p>
                  <p className="mt-0.5 text-[11px] text-[#6a6a6a]">
                    {timeAgo(block.timestamp)}
                  </p>
                </div>
              </Link>
            ))}
          </div>
        </DappPanel>

        <DappPanel className="overflow-hidden">
          <div className="flex items-center justify-between border-b border-[#0a0a0a]/10 px-5 py-4">
            <span className="text-base font-semibold">
              Latest transactions
            </span>
            <Link
              className="text-xs text-[#6a6a6a] hover:text-[#0a0a0a]"
              href="/blockexplorer/txs"
            >
              View all txs →
            </Link>
          </div>
          <div className="flex gap-1 border-b border-[#0a0a0a]/10 px-4 py-3">
            {filters.map((item) => (
              <button
                className="rounded-full px-2.5 py-1 text-[11px] font-semibold capitalize transition hover:bg-[#f5f0e0] data-[active=true]:bg-[#0a0a0a] data-[active=true]:text-white"
                data-active={filter === item}
                key={item}
                onClick={() => setFilter(item)}
                type="button"
              >
                {item}
              </button>
            ))}
          </div>
          <div className="divide-y divide-[#0a0a0a]/10">
            {explorerTxs.length === 0 && (
              <p className="px-5 py-4 text-sm text-[#6a6a6a]">
                No transactions yet...
              </p>
            )}
            {explorerTxs.slice(0, 8).map((tx) => (
              <Link
                className="grid grid-cols-[auto_1fr_auto] items-center gap-3 px-4 py-3 transition hover:bg-[#f5f0e0]"
                href={`/blockexplorer/tx/${tx.hash}`}
                key={tx.hash}
              >
                <TransactionTypeBadge type={tx.type} />
                <div className="min-w-0">
                  <p className="truncate text-[13px]">
                    <Mono>{shortHash(tx.hash)}</Mono>
                  </p>
                  <p className="mt-0.5 truncate text-[11px] text-[#6a6a6a]">
                    <Mono>{shortHash(tx.from)}</Mono> →{" "}
                    <Mono>{shortHash(tx.to)}</Mono>
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-[13px] font-semibold">
                    <Mono>{tx.value}</Mono> IPT
                  </p>
                </div>
              </Link>
            ))}
          </div>
        </DappPanel>
      </div>
      )}
    </div>
  );
}
