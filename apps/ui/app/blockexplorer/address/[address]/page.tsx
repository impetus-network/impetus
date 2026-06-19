"use client";

import { useState } from "react";
import { useParams } from "next/navigation";
import { useAddressBalance, useAddressTxs } from "~/hooks/useExplorer";
import {
  Card,
  DetailRow,
  ExplorerPage,
  Pager,
  TxRow,
  formatIpt,
} from "~/components/blockexplorer/ExplorerUI";

const PAGE = 25;

export default function AddressPage() {
  const { address } = useParams<{ address: string }>();
  const addr = address.toLowerCase();
  const [offset, setOffset] = useState(0);
  const { data: balance } = useAddressBalance(addr);
  const { data: txData, isLoading } = useAddressTxs(addr, PAGE, offset);

  const txs = txData?.evmTransactions ?? [];
  const total = txData?.evmTransactionsConnection.totalCount;

  return (
    <ExplorerPage>
      <h1 className="text-2xl font-bold">Address</h1>

      <Card>
        <dl>
          <DetailRow label="Address">{address}</DetailRow>
          <DetailRow label="Balance">
            <span className="text-base font-semibold">
              {formatIpt(balance?.free ?? "0")} IPT
            </span>
          </DetailRow>
          {balance && BigInt(balance.frozen) > 0n && (
            <DetailRow label="Locked">{formatIpt(balance.frozen)} IPT</DetailRow>
          )}
          {balance && BigInt(balance.reserved) > 0n && (
            <DetailRow label="Reserved">{formatIpt(balance.reserved)} IPT</DetailRow>
          )}
          <DetailRow label="Nonce">{balance?.nonce ?? 0}</DetailRow>
        </dl>
      </Card>

      <Card>
        <h2 className="mb-2 text-base font-semibold">
          Transactions {total != null ? `(${total.toLocaleString()})` : ""}
        </h2>
        {isLoading ? (
          <p className="py-2 text-sm text-muted-foreground">Loading…</p>
        ) : txs.length === 0 ? (
          <p className="py-2 text-sm text-muted-foreground">
            No transactions for this address.
          </p>
        ) : (
          <>
            <div>
              {txs.map((tx) => (
                <TxRow key={tx.id} tx={tx} context={addr} />
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
