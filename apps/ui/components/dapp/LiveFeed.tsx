"use client";

import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { formatEther } from "viem";
import { useBlockNumber, usePublicClient } from "wagmi";
import { Mono, PulseDot } from "./DappPrimitives";
import { shortHash } from "./mockData";
import type { LiveFeedState, MockTransaction } from "./types";

function classifyTx(to: string | null): MockTransaction["type"] {
  if (!to) return "contract";
  const last = parseInt(to.slice(-1), 16);
  if (last < 2) return "swap";
  if (last < 4) return "mint";
  if (last < 6) return "contract";
  return "transfer";
}

export function useLiveFeed(): LiveFeedState {
  const publicClient = usePublicClient();
  const { data: latestBlockNumber } = useBlockNumber({ watch: true });
  const [feed, setFeed] = useState<LiveFeedState>({
    block: 0,
    gasPrice: 0,
    tps: 0,
    txs: [],
  });
  const seenBlockRef = useRef<bigint>(0n);

  useEffect(() => {
    if (!publicClient || latestBlockNumber === undefined) return;
    if (latestBlockNumber === seenBlockRef.current) return;
    seenBlockRef.current = latestBlockNumber;

    async function fetchLatest() {
      const block = await publicClient!.getBlock({
        blockNumber: latestBlockNumber!,
        includeTransactions: true,
      });

      const txs: MockTransaction[] = (
        block.transactions as Array<{
          hash: `0x${string}`;
          from: `0x${string}`;
          to: `0x${string}` | null;
          value: bigint;
        }>
      )
        .slice(0, 8)
        .map((tx, i) => ({
          hash: tx.hash,
          from: tx.from,
          to: (tx.to ?? tx.from) as `0x${string}`,
          value: Number(formatEther(tx.value)).toFixed(3),
          type: classifyTx(tx.to),
          age: i * 2,
        }));

      const txCount = block.transactions.length;
      let tps = 0;
      if (latestBlockNumber! > 0n) {
        try {
          const prevBlock = await publicClient!.getBlock({
            blockNumber: latestBlockNumber! - 1n,
          });
          const elapsed = Number(block.timestamp - prevBlock.timestamp);
          tps = elapsed > 0 ? Math.round(txCount / elapsed) : txCount;
        } catch {
          tps = txCount;
        }
      }

      setFeed((prev) => {
        const existingHashes = new Set(txs.map((t) => t.hash));
        const carryOver = prev.txs
          .filter((t) => !existingHashes.has(t.hash))
          .map((t) => ({ ...t, age: t.age + 6 }));

        return {
          block: Number(latestBlockNumber!),
          gasPrice: 0,
          tps,
          txs: [...txs, ...carryOver].slice(0, 8),
        };
      });
    }

    fetchLatest();
  }, [publicClient, latestBlockNumber]);

  return feed;
}

const txDotColor: Record<string, string> = {
  transfer: "#a4d4c5",
  swap: "#ff4d8b",
  mint: "#b8a4ed",
  contract: "#e8b94a",
};

export function NetworkFeedCard({
  feed,
}: {
  feed: LiveFeedState;
}): ReactElement {
  return (
    <section className="flex min-h-[420px] flex-col gap-4 rounded-3xl bg-[#1a3a3a] p-7 text-white">
      <div className="flex items-center justify-between">
        <p className="art-caption text-white/70">Network feed</p>
        <div className="flex items-center gap-2 text-xs">
          <PulseDot color="#a4d4c5" />
          <span className="text-[#a4d4c5]">Synced</span>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div>
          <p className="text-xs text-white/60">Block</p>
          <p className="mt-1.5 font-mono text-[22px] font-medium">
            #{feed.block.toLocaleString("en-US")}
          </p>
        </div>
        <div>
          <p className="text-xs text-white/60">Throughput</p>
          <p className="mt-1.5 font-mono text-[22px] font-medium">
            {feed.tps.toLocaleString("en-US")}{" "}
            <span className="text-sm text-white/60">tps</span>
          </p>
        </div>
      </div>

      <div className="h-px bg-white/10" />

      <div className="flex flex-1 flex-col gap-2">
        <p className="art-caption mb-1 text-[10px] text-white/50">
          Recent transactions
        </p>
        {feed.txs.length === 0 && (
          <p className="text-xs text-white/40">Waiting for transactions...</p>
        )}
        {feed.txs.slice(0, 5).map((tx, index) => (
          <div
            className="grid grid-cols-[12px_1fr_auto] items-center gap-2.5 text-xs"
            key={tx.hash}
            style={{ opacity: 1 - index * 0.12 }}
          >
            <span
              className="size-1.5 rounded-full"
              style={{ backgroundColor: txDotColor[tx.type] ?? "#a4d4c5" }}
            />
            <Mono className="truncate">{shortHash(tx.hash)}</Mono>
            <Mono className="text-white/70">{tx.value} ART</Mono>
          </div>
        ))}
      </div>

      <div className="mt-auto flex items-center justify-between border-t border-white/10 pt-3.5 text-xs text-[#a4d4c5]">
        <span>Gas fee</span>
        <Mono className="font-medium">0.000 ART · gasless</Mono>
      </div>
    </section>
  );
}
