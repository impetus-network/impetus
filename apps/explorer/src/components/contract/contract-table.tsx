import { formatNumber } from "@artemis/shared";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@artemis/coss-ui/ui/table";
import { Badge } from "@artemis/coss-ui/ui/badge";
import { AddressLink } from "@/components/shared/address-link";

interface ContractSummary {
  address: string;
  name: string | null;
  txCount: number;
  verified: boolean;
}

interface ContractTableProps {
  items: ReadonlyArray<ContractSummary>;
}

export function ContractTable({ items }: ContractTableProps) {
  if (items.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500">No contracts found.</p>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Address</TableHead>
          <TableHead>Name</TableHead>
          <TableHead className="text-right">Txns</TableHead>
          <TableHead>Verified</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((contract) => (
          <TableRow key={contract.address}>
            <TableCell>
              <AddressLink address={contract.address} head={10} tail={8} />
            </TableCell>
            <TableCell>
              {contract.name ?? (
                <span className="text-gray-400">--</span>
              )}
            </TableCell>
            <TableCell className="text-right tabular-nums">
              {formatNumber(contract.txCount)}
            </TableCell>
            <TableCell>
              {contract.verified ? (
                <Badge variant="success" size="sm">
                  Verified
                </Badge>
              ) : (
                <Badge variant="outline" size="sm">
                  Unverified
                </Badge>
              )}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
