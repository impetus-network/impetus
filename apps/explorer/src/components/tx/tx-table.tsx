import Link from "next/link";
import { formatRelativeTime, formatBalance } from "@artemis/shared";
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

interface TxSummary {
  hash: string;
  blockNum: number;
  from: string;
  to: string | null;
  value: string;
  timestamp: number;
}

interface TxTableProps {
  items: ReadonlyArray<TxSummary>;
}

export function TxTable({ items }: TxTableProps) {
  if (items.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500">
        No transactions found.
      </p>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Tx Hash</TableHead>
          <TableHead>Block</TableHead>
          <TableHead>From</TableHead>
          <TableHead>To</TableHead>
          <TableHead className="text-right">Value (ART)</TableHead>
          <TableHead>Age</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((tx) => (
          <TableRow key={tx.hash}>
            <TableCell>
              <HashLink hash={tx.hash} href={`/tx/${tx.hash}`} />
            </TableCell>
            <TableCell>
              <Link
                href={`/block/${tx.blockNum}`}
                className="text-blue-600 hover:text-blue-800 hover:underline"
              >
                {tx.blockNum}
              </Link>
            </TableCell>
            <TableCell>
              <AddressLink address={tx.from} />
            </TableCell>
            <TableCell>
              {tx.to ? (
                <AddressLink address={tx.to} />
              ) : (
                <span className="text-xs text-gray-400 italic">
                  Contract Creation
                </span>
              )}
            </TableCell>
            <TableCell className="text-right tabular-nums">
              {formatBalance(tx.value, 18)}
            </TableCell>
            <TableCell className="text-gray-500">
              {formatRelativeTime(tx.timestamp)}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
