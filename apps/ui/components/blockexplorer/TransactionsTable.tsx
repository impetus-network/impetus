"use client";

import type { Transaction } from "viem";
import { formatEther } from "viem";
import { Address } from "~/components/scaffold/Address";
import { ClayEmptyState, ClayTableFrame } from "@artemis/coss-ui/clay";

interface TransactionsTableProps {
  transactions: Transaction[];
  loading: boolean;
}

export function TransactionsTable({ transactions, loading }: TransactionsTableProps) {
  if (loading) {
    return (
      <ClayEmptyState
        title="Loading transactions"
        description="Fetching recent transactions from the latest blocks."
      />
    );
  }
  if (transactions.length === 0) {
    return (
      <ClayEmptyState
        title="No transactions found"
        description="Transactions from recent blocks will appear here."
      />
    );
  }

  return (
    <ClayTableFrame>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted text-left">
            <th className="px-4 py-3">Tx Hash</th>
            <th className="px-4 py-3">From</th>
            <th className="px-4 py-3">To</th>
            <th className="px-4 py-3">Value</th>
          </tr>
        </thead>
        <tbody>
          {transactions.map((tx) => (
            <tr key={tx.hash} className="border-b border-border hover:bg-muted/50">
              <td className="px-4 py-3 font-mono text-xs">
                {tx.hash.slice(0, 10)}...{tx.hash.slice(-6)}
              </td>
              <td className="px-4 py-3">
                <Address address={tx.from} />
              </td>
              <td className="px-4 py-3">
                {tx.to ? <Address address={tx.to} /> : <span className="text-muted-foreground">Contract Create</span>}
              </td>
              <td className="px-4 py-3 font-mono">
                {Number(formatEther(tx.value)).toFixed(4)} IPT
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </ClayTableFrame>
  );
}
