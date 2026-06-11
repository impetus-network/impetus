"use client";

import { trpc } from "@/trpc/client";
import { formatHex, formatRelativeTime, formatBalance } from "@artemis/shared";
import Link from "next/link";

export function LatestTxs() {
  const { data, isLoading } = trpc.tx.latest.useQuery(
    { limit: 10 },
    { refetchInterval: 6000, staleTime: 6000 },
  );

  if (isLoading || !data) {
    return (
      <div className="animate-pulse space-y-2">
        {Array.from({ length: 5 }, (_, i) => (
          <div key={i} className="h-14 rounded-lg bg-gray-100" />
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {data.map((tx) => (
        <Link
          key={tx.hash}
          href={`/tx/${tx.hash}`}
          className="flex items-center justify-between rounded-lg border p-3 hover:bg-gray-50 transition-colors"
        >
          <div className="min-w-0">
            <span className="font-mono text-sm text-blue-600">
              {formatHex(tx.hash as `0x${string}`, 10, 6)}
            </span>
            <span className="ml-2 text-xs text-gray-500">
              {formatRelativeTime(tx.timestamp)}
            </span>
          </div>
          <span className="shrink-0 text-xs text-gray-500">
            {formatBalance(tx.value, 18)} ART
          </span>
        </Link>
      ))}
    </div>
  );
}
