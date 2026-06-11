"use client";

import { trpc } from "@/trpc/client";
import { formatRelativeTime } from "@artemis/shared";
import Link from "next/link";

export function LatestBlocks() {
  const { data, isLoading } = trpc.block.latest.useQuery(
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
      {data.map((block) => (
        <Link
          key={block.blockNum}
          href={`/block/${block.blockNum}`}
          className="flex items-center justify-between rounded-lg border p-3 hover:bg-gray-50 transition-colors"
        >
          <div>
            <span className="font-mono font-medium text-sm">
              #{block.blockNum}
            </span>
            <span className="ml-2 text-xs text-gray-500">
              {formatRelativeTime(block.timestamp)}
            </span>
          </div>
          <span className="text-xs text-gray-500">{block.txCount} txs</span>
        </Link>
      ))}
    </div>
  );
}
