import Link from "next/link";
import { formatRelativeTime } from "@artemis/shared";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@artemis/coss-ui/ui/table";
import { HashLink } from "@/components/shared/hash-link";
import { AddressLink } from "@/components/shared/address-link";

interface BlockSummary {
  blockNum: number;
  hash: string;
  miner: string;
  timestamp: number;
  txCount: number;
}

interface BlockTableProps {
  items: ReadonlyArray<BlockSummary>;
}

export function BlockTable({ items }: BlockTableProps) {
  if (items.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500">No blocks found.</p>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Block</TableHead>
          <TableHead>Hash</TableHead>
          <TableHead>Miner</TableHead>
          <TableHead>Age</TableHead>
          <TableHead className="text-right">Txns</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((block) => (
          <TableRow key={block.blockNum}>
            <TableCell>
              <Link
                href={`/block/${block.blockNum}`}
                className="font-medium text-blue-600 hover:text-blue-800 hover:underline"
              >
                {block.blockNum}
              </Link>
            </TableCell>
            <TableCell>
              <HashLink
                hash={block.hash}
                href={`/block/${block.blockNum}`}
              />
            </TableCell>
            <TableCell>
              {block.miner ? (
                <AddressLink address={block.miner} />
              ) : (
                <span className="text-gray-400">--</span>
              )}
            </TableCell>
            <TableCell className="text-gray-500">
              {formatRelativeTime(block.timestamp)}
            </TableCell>
            <TableCell className="text-right tabular-nums">
              {block.txCount}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
