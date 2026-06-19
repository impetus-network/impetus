"use client";

import { useState } from "react";
import Link from "next/link";
import { useExplorerBlocks } from "~/hooks/useExplorer";
import {
  AddrLink,
  Card,
  Pager,
  timeAgo,
} from "~/components/blockexplorer/ExplorerUI";

const PAGE = 25;

export default function BlocksPage() {
  const [offset, setOffset] = useState(0);
  const { data, isLoading } = useExplorerBlocks(PAGE, offset);
  const blocks = data?.blocks ?? [];
  const total = data?.blocksConnection.totalCount;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Blocks</h1>
      <Card>
        {isLoading ? (
          <p className="py-2 text-sm text-muted-foreground">Loading…</p>
        ) : (
          <>
            <div className="hidden grid-cols-[120px_1fr_auto_auto] gap-4 border-b border-border pb-2 text-xs uppercase text-muted-foreground sm:grid">
              <span>Block</span>
              <span>Validator</span>
              <span>Txs</span>
              <span className="text-right">Age</span>
            </div>
            {blocks.map((b) => (
              <article
                className="grid grid-cols-[1fr_auto] items-center gap-3 border-b border-border/60 py-3 last:border-0 sm:grid-cols-[120px_1fr_auto_auto] sm:gap-4"
                key={b.id}
              >
                <Link
                  className="font-mono text-sm text-primary hover:underline"
                  href={`/blockexplorer/block/${b.height}`}
                >
                  #{b.height.toLocaleString()}
                </Link>
                <span className="hidden truncate font-mono text-sm sm:block">
                  <AddrLink address={b.author} />
                </span>
                <span className="hidden text-sm sm:block">{b.txCount} txs</span>
                <span className="text-right text-xs text-muted-foreground">
                  {timeAgo(b.timestamp)}
                </span>
              </article>
            ))}
            <Pager
              limit={PAGE}
              offset={offset}
              onNext={() => setOffset((o) => o + PAGE)}
              onPrev={() => setOffset((o) => Math.max(0, o - PAGE))}
              total={total}
            />
          </>
        )}
      </Card>
    </div>
  );
}
