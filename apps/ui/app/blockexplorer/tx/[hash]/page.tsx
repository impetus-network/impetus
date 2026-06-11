"use client";

import { useParams } from "next/navigation";
import { useTransaction, useTransactionReceipt } from "wagmi";
import { formatEther, type Hash } from "viem";
import { Address } from "~/components/scaffold/Address";

export default function TxPage() {
  const { hash } = useParams<{ hash: string }>();
  const { data: tx, isLoading } = useTransaction({ hash: hash as Hash });
  const { data: receipt } = useTransactionReceipt({ hash: hash as Hash });

  if (isLoading) return <p className="text-muted-foreground">Loading...</p>;
  if (!tx) return <p className="text-muted-foreground">Transaction not found.</p>;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Transaction</h1>
      <div className="rounded-lg border border-border p-6">
        <dl className="grid gap-4">
          <div>
            <dt className="text-sm text-muted-foreground">Hash</dt>
            <dd className="break-all font-mono text-sm">{tx.hash}</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Status</dt>
            <dd>
              {receipt?.status === "success" ? (
                <span className="font-medium text-success">Success</span>
              ) : receipt?.status === "reverted" ? (
                <span className="font-medium text-destructive">Reverted</span>
              ) : (
                <span className="text-muted-foreground">Pending</span>
              )}
            </dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">From</dt>
            <dd><Address address={tx.from} /></dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">To</dt>
            <dd>{tx.to ? <Address address={tx.to} /> : "Contract Creation"}</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Value</dt>
            <dd className="font-mono">{formatEther(tx.value)} ART</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Gas Used</dt>
            <dd className="font-mono">{receipt?.gasUsed?.toString() ?? "—"}</dd>
          </div>
          <div>
            <dt className="text-sm text-muted-foreground">Block</dt>
            <dd className="font-mono">{tx.blockNumber?.toString()}</dd>
          </div>
        </dl>
      </div>
    </div>
  );
}
