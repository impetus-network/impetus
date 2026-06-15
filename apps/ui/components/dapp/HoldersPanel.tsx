"use client";

import { type ReactElement } from "react";
import { formatEther } from "viem";
import { DappPanel, Mono } from "./DappPrimitives";
import { shortHash } from "./mockData";
import { useHolders } from "~/hooks/useHolders";

function formatBalance(raw: string): string {
  const value = Number(formatEther(BigInt(raw)));
  return value.toLocaleString("en-US", {
    maximumFractionDigits: value >= 1 ? 2 : 6,
  });
}

function percentOfSupply(total: string, issuance: string): string {
  const supply = BigInt(issuance);
  if (supply === 0n) return "—";
  // basis points for sub-percent precision without floats on bigint
  const bps = Number((BigInt(total) * 1_000_000n) / supply) / 10_000;
  return `${bps.toFixed(bps >= 1 ? 2 : 4)}%`;
}

export function HoldersPanel(): ReactElement {
  const { data, isLoading } = useHolders(50);
  const holders = data?.holders ?? [];
  const issuance = data?.totalIssuance ?? "0";

  return (
    <DappPanel className="overflow-hidden">
      <div className="flex items-center justify-between border-b border-[#0a0a0a]/10 px-5 py-4">
        <span className="text-base font-semibold">Top holders</span>
        <span className="text-xs text-[#6a6a6a]">
          {data ? `${data.holdersCount.toLocaleString()} holders` : "…"}
        </span>
      </div>

      <div className="grid grid-cols-[auto_1fr_auto_auto] items-center gap-3 border-b border-[#0a0a0a]/10 px-4 py-2.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#6a6a6a]">
        <span>#</span>
        <span>Address</span>
        <span className="text-right">Balance (IPT)</span>
        <span className="text-right">Supply</span>
      </div>

      <div className="divide-y divide-[#0a0a0a]/10">
        {isLoading && holders.length === 0 && (
          <p className="px-5 py-4 text-sm text-[#6a6a6a]">Loading holders…</p>
        )}
        {!isLoading && holders.length === 0 && (
          <p className="px-5 py-4 text-sm text-[#6a6a6a]">No holders indexed yet.</p>
        )}
        {holders.map((holder, index) => (
          <article
            className="grid grid-cols-[auto_1fr_auto_auto] items-center gap-3 px-4 py-3"
            key={holder.id}
          >
            <span className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-[#f5f0e0] font-mono text-[11px] text-[#6a6a6a]">
              {index + 1}
            </span>
            <p className="min-w-0 truncate text-[13px]">
              <Mono>{shortHash(holder.id)}</Mono>
            </p>
            <p className="text-right text-[13px] font-semibold">
              <Mono>{formatBalance(holder.total)}</Mono>
            </p>
            <p className="text-right text-[12px] text-[#6a6a6a]">
              {percentOfSupply(holder.total, issuance)}
            </p>
          </article>
        ))}
      </div>
    </DappPanel>
  );
}
