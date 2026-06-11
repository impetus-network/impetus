import { formatTimestamp, formatNumber } from "@artemis/shared";
import { Separator } from "@artemis/coss-ui/ui/separator";
import { AddressLink } from "@/components/shared/address-link";
import { CopyButton } from "@/components/shared/copy-button";
import { TimestampClient } from "@/components/shared/timestamp-client";

interface BlockDetail {
  blockNum: number;
  hash: string;
  parentHash: string;
  miner: string;
  timestamp: number;
  txCount: number;
  gasUsed: string;
  gasLimit: string;
  size: string;
}

interface BlockInfoPanelProps {
  block: BlockDetail;
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

export function BlockInfoPanel({ block }: BlockInfoPanelProps) {
  return (
    <dl className="divide-y divide-gray-100">
      <Row label="Block Number">{formatNumber(block.blockNum)}</Row>
      <Row label="Timestamp">
        <TimestampClient unixSec={block.timestamp} />
        <span className="ml-2 text-xs text-gray-400">
          ({formatTimestamp(block.timestamp)})
        </span>
      </Row>
      <Row label="Transactions">{block.txCount} transactions</Row>
      <Separator />
      <Row label="Hash">
        <span className="font-mono text-xs">{block.hash}</span>
        <CopyButton text={block.hash} />
      </Row>
      <Row label="Parent Hash">
        <span className="font-mono text-xs">{block.parentHash}</span>
        <CopyButton text={block.parentHash} />
      </Row>
      <Row label="Miner">
        {block.miner ? (
          <AddressLink address={block.miner} head={10} tail={8} />
        ) : (
          <span className="text-gray-400">--</span>
        )}
      </Row>
      <Separator />
      <Row label="Gas Used">{formatNumber(block.gasUsed)}</Row>
      <Row label="Gas Limit">{formatNumber(block.gasLimit)}</Row>
      <Row label="Size">{formatNumber(block.size)} bytes</Row>
    </dl>
  );
}
