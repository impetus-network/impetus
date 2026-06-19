"use client";

import { useParams } from "next/navigation";
import Link from "next/link";
import { useExplorerBlock, useExplorerBlockTxs } from "~/hooks/useExplorer";
import {
  AddrLink,
  Card,
  DetailRow,
  TxRow,
  formatIpt,
  shorten,
  timeAgo,
} from "~/components/blockexplorer/ExplorerUI";

export default function BlockPage() {
  const { height } = useParams<{ height: string }>();
  const num = Number(height);
  const { data: block, isLoading } = useExplorerBlock(num);
  const { data: txs } = useExplorerBlockTxs(num);

  if (isLoading) return <p className="text-muted-foreground">Loading block…</p>;
  if (!block)
    return (
      <p className="text-muted-foreground">
        Block #{height} not indexed yet.
      </p>
    );

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-bold">Block #{block.height.toLocaleString()}</h1>
        <div className="ml-auto flex gap-2 text-sm">
          <Link
            className="rounded-lg border border-border px-3 py-1.5 disabled:opacity-40"
            href={`/blockexplorer/block/${block.height - 1}`}
          >
            ← Prev
          </Link>
          <Link
            className="rounded-lg border border-border px-3 py-1.5"
            href={`/blockexplorer/block/${block.height + 1}`}
          >
            Next →
          </Link>
        </div>
      </div>

      <Card>
        <dl>
          <DetailRow label="Height">{block.height.toLocaleString()}</DetailRow>
          <DetailRow label="Timestamp">
            {new Date(block.timestamp).toUTCString()} ({timeAgo(block.timestamp)})
          </DetailRow>
          <DetailRow label="Hash">{block.hash}</DetailRow>
          <DetailRow label="Parent hash">
            <Link
              className="text-primary hover:underline"
              href={`/blockexplorer/block/${block.height - 1}`}
            >
              {block.parentHash}
            </Link>
          </DetailRow>
          <DetailRow label="Validator">
            <AddrLink address={block.author} />
          </DetailRow>
          <DetailRow label="Transactions">{block.txCount}</DetailRow>
          <DetailRow label="Gas used">
            {BigInt(block.gasUsed).toLocaleString()} / {BigInt(block.gasLimit).toLocaleString()}
          </DetailRow>
          {block.baseFeePerGas && (
            <DetailRow label="Base fee">{formatIpt(block.baseFeePerGas)} IPT</DetailRow>
          )}
          <DetailRow label="Size">{BigInt(block.size).toLocaleString()} bytes</DetailRow>
        </dl>
      </Card>

      <Card>
        <h2 className="mb-2 text-base font-semibold">
          Transactions {txs ? `(${txs.length})` : ""}
        </h2>
        {!txs || txs.length === 0 ? (
          <p className="py-2 text-sm text-muted-foreground">
            No transactions in this block.
          </p>
        ) : (
          <div>
            {txs.map((tx) => (
              <TxRow key={tx.id} tx={tx} />
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
