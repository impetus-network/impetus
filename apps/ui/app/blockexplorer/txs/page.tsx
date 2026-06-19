"use client";

import { useState } from "react";
import { useExplorerTxs } from "~/hooks/useExplorer";
import {
  Card,
  ExplorerPage,
  ListRowsSkeleton,
  Pager,
  TxRow,
} from "~/components/blockexplorer/ExplorerUI";

const PAGE = 25;

export default function TxsPage() {
  const [offset, setOffset] = useState(0);
  const { data, isLoading } = useExplorerTxs(PAGE, offset);
  const txs = data?.evmTransactions ?? [];
  const total = data?.evmTransactionsConnection.totalCount;

  return (
    <ExplorerPage>
      <h1 className="text-2xl font-bold">Transactions</h1>
      <Card>
        {isLoading ? (
          <ListRowsSkeleton rows={12} />
        ) : txs.length === 0 ? (
          <p className="py-2 text-sm text-muted-foreground">
            No transactions indexed yet.
          </p>
        ) : (
          <>
            <div>
              {txs.map((tx) => (
                <TxRow key={tx.id} tx={tx} />
              ))}
            </div>
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
    </ExplorerPage>
  );
}
