"use client";

import Link from "next/link";
import { formatEther } from "viem";
import type { ReactNode } from "react";
import type { ExplorerTx } from "~/hooks/useExplorer";

export function shorten(value: string, head = 8, tail = 6): string {
  if (value.length <= head + tail) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}

/** wei string -> trimmed IPT amount. */
export function formatIpt(wei: string | null | undefined): string {
  if (!wei) return "0";
  const s = formatEther(BigInt(wei));
  return s.includes(".") ? s.replace(/\.?0+$/, "") : s;
}

export function timeAgo(iso: string): string {
  const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (diff < 2) return "just now";
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function txFeeIpt(tx: { gasUsed: string; effectiveGasPrice: string }): string {
  return formatIpt((BigInt(tx.gasUsed) * BigInt(tx.effectiveGasPrice)).toString());
}

export function Card({ children }: { children: ReactNode }) {
  return <div className="rounded-2xl border border-border bg-card p-6">{children}</div>;
}

export function DetailRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-1 border-b border-border/60 py-3 last:border-0 sm:grid-cols-[180px_1fr] sm:gap-4">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="break-all font-mono text-sm">{children}</dd>
    </div>
  );
}

export function AddrLink({ address }: { address: string | null }) {
  if (!address) return <span className="text-muted-foreground">Contract creation</span>;
  return (
    <Link className="text-primary hover:underline" href={`/blockexplorer/address/${address}`}>
      {address}
    </Link>
  );
}

export function StatusBadge({ success }: { success: boolean }) {
  return success ? (
    <span className="rounded-md bg-success/10 px-2 py-0.5 text-xs font-medium text-success">
      Success
    </span>
  ) : (
    <span className="rounded-md bg-destructive/10 px-2 py-0.5 text-xs font-medium text-destructive">
      Failed
    </span>
  );
}

export function Pager({
  offset,
  limit,
  total,
  onPrev,
  onNext,
}: {
  offset: number;
  limit: number;
  total: number | undefined;
  onPrev: () => void;
  onNext: () => void;
}) {
  const from = total === 0 ? 0 : offset + 1;
  const to = Math.min(offset + limit, total ?? offset + limit);
  const hasNext = total != null ? offset + limit < total : false;
  return (
    <div className="flex items-center justify-between pt-4 text-sm">
      <span className="text-muted-foreground">
        {from.toLocaleString()}–{to.toLocaleString()}
        {total != null ? ` of ${total.toLocaleString()}` : ""}
      </span>
      <div className="flex gap-2">
        <button
          className="rounded-lg border border-border px-3 py-1.5 disabled:opacity-40"
          disabled={offset === 0}
          onClick={onPrev}
          type="button"
        >
          ← Prev
        </button>
        <button
          className="rounded-lg border border-border px-3 py-1.5 disabled:opacity-40"
          disabled={!hasNext}
          onClick={onNext}
          type="button"
        >
          Next →
        </button>
      </div>
    </div>
  );
}

/** A compact transaction row used in block / address / list views. */
export function TxRow({ tx, context }: { tx: ExplorerTx; context?: string }) {
  return (
    <article className="grid grid-cols-[1fr_auto] items-center gap-3 border-b border-border/60 py-3 last:border-0">
      <div className="min-w-0">
        <Link
          className="truncate font-mono text-sm text-primary hover:underline"
          href={`/blockexplorer/tx/${tx.id}`}
        >
          {shorten(tx.id, 12, 8)}
        </Link>
        <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
          {context === tx.from ? "OUT " : context && context === tx.to ? "IN  " : ""}
          {shorten(tx.from)} → {tx.to ? shorten(tx.to) : "contract creation"}
        </p>
      </div>
      <div className="text-right">
        <p className="font-mono text-sm">{formatIpt(tx.value)} IPT</p>
        <p className="mt-0.5 text-xs text-muted-foreground">
          <Link className="hover:underline" href={`/blockexplorer/block/${tx.block}`}>
            #{tx.block.toLocaleString()}
          </Link>{" "}
          · {timeAgo(tx.timestamp)}
        </p>
      </div>
    </article>
  );
}
