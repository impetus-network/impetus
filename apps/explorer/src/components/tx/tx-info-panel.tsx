import Link from "next/link";
import {
  formatTimestamp,
  formatBalance,
  formatNumber,
} from "@artemis/shared";
import { Badge } from "@artemis/coss-ui/ui/badge";
import { Separator } from "@artemis/coss-ui/ui/separator";
import { AddressLink } from "@/components/shared/address-link";
import { CopyButton } from "@/components/shared/copy-button";
import { TimestampClient } from "@/components/shared/timestamp-client";
import { TxInputData } from "@/components/tx/tx-input-data";

interface TxDetail {
  hash: string;
  blockNum: number;
  from: string;
  to: string | null;
  value: string;
  timestamp: number;
  status: "Success" | "Failed";
  nonce: number;
  inputData: string;
  fee: string;
  gasUsed: string;
  gasPrice: string;
}

interface TxInfoPanelProps {
  tx: TxDetail;
}

function Row({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-4 py-3">
      <dt className="w-40 shrink-0 text-sm font-medium text-gray-500">
        {label}
      </dt>
      <dd className="text-sm text-gray-900 break-all">{children}</dd>
    </div>
  );
}

export function TxInfoPanel({ tx }: TxInfoPanelProps) {
  return (
    <dl className="divide-y divide-gray-100">
      <Row label="Transaction Hash">
        <span className="font-mono text-xs">{tx.hash}</span>
        <CopyButton text={tx.hash} />
      </Row>
      <Row label="Status">
        <Badge
          variant={tx.status === "Success" ? "success" : "destructive"}
        >
          {tx.status}
        </Badge>
      </Row>
      <Row label="Block">
        <Link
          href={`/block/${tx.blockNum}`}
          className="text-blue-600 hover:text-blue-800 hover:underline"
        >
          {formatNumber(tx.blockNum)}
        </Link>
      </Row>
      <Row label="Timestamp">
        <TimestampClient unixSec={tx.timestamp} />
        <span className="ml-2 text-xs text-gray-400">
          ({formatTimestamp(tx.timestamp)})
        </span>
      </Row>
      <Separator />
      <Row label="From">
        <AddressLink address={tx.from} head={10} tail={8} />
      </Row>
      <Row label="To">
        {tx.to ? (
          <AddressLink address={tx.to} head={10} tail={8} />
        ) : (
          <span className="text-gray-400 italic">Contract Creation</span>
        )}
      </Row>
      <Separator />
      <Row label="Value">{formatBalance(tx.value, 18)} ART</Row>
      <Row label="Transaction Fee">{formatBalance(tx.fee, 18)} ART</Row>
      <Row label="Gas Used">{formatNumber(tx.gasUsed)}</Row>
      <Row label="Gas Price">{formatNumber(tx.gasPrice)} wei</Row>
      <Row label="Nonce">{tx.nonce}</Row>
      <Separator />
      <Row label="Input Data">
        <TxInputData data={tx.inputData} />
      </Row>
    </dl>
  );
}
