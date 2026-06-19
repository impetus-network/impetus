"use client";

import { useParams } from "next/navigation";
import Link from "next/link";
import { useExplorerTx } from "~/hooks/useExplorer";
import {
  AddrLink,
  Card,
  DetailRow,
  ExplorerPage,
  StatusBadge,
  formatIpt,
  timeAgo,
  txFeeIpt,
} from "~/components/blockexplorer/ExplorerUI";

export default function TxPage() {
  const { hash } = useParams<{ hash: string }>();
  const { data: tx, isLoading } = useExplorerTx(hash);

  if (isLoading)
    return (
      <ExplorerPage>
        <p className="text-muted-foreground">Loading transaction…</p>
      </ExplorerPage>
    );
  if (!tx)
    return (
      <ExplorerPage>
        <p className="text-muted-foreground">
          Transaction not indexed (yet). It may be pending or out of the synced
          range.
        </p>
      </ExplorerPage>
    );

  return (
    <ExplorerPage>
      <h1 className="text-2xl font-bold">Transaction</h1>
      <Card>
        <dl>
          <DetailRow label="Hash">{tx.id}</DetailRow>
          <DetailRow label="Status">
            <StatusBadge success={tx.success} />
          </DetailRow>
          <DetailRow label="Block">
            <Link
              className="text-primary hover:underline"
              href={`/blockexplorer/block/${tx.block}`}
            >
              #{tx.block.toLocaleString()}
            </Link>
          </DetailRow>
          <DetailRow label="Timestamp">
            {new Date(tx.timestamp).toUTCString()} ({timeAgo(tx.timestamp)})
          </DetailRow>
          <DetailRow label="From">
            <AddrLink address={tx.from} />
          </DetailRow>
          <DetailRow label="To">
            {tx.to ? (
              <AddrLink address={tx.to} />
            ) : tx.contractCreated ? (
              <span>
                Contract created: <AddrLink address={tx.contractCreated} />
              </span>
            ) : (
              <span className="text-muted-foreground">Contract creation</span>
            )}
          </DetailRow>
          <DetailRow label="Value">{formatIpt(tx.value)} IPT</DetailRow>
          <DetailRow label="Transaction fee">{txFeeIpt(tx)} IPT</DetailRow>
          <DetailRow label="Gas used">
            {BigInt(tx.gasUsed).toLocaleString()}
          </DetailRow>
          <DetailRow label="Effective gas price">
            {BigInt(tx.effectiveGasPrice).toLocaleString()} wei
          </DetailRow>
          <DetailRow label="Nonce">{tx.nonce}</DetailRow>
          {tx.txType != null && <DetailRow label="Type">{tx.txType}</DetailRow>}
          <DetailRow label="Input data">
            <span className="block max-h-40 overflow-auto break-all text-xs">
              {tx.input === "0x" ? "0x (none)" : tx.input}
            </span>
          </DetailRow>
        </dl>
      </Card>
    </ExplorerPage>
  );
}
