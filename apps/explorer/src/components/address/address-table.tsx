import { formatBalance } from "@artemis/shared";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@artemis/coss-ui/ui/table";
import { AddressLink } from "@/components/shared/address-link";

interface AccountSummary {
  address: string;
  balance: string;
}

interface AddressTableProps {
  items: ReadonlyArray<AccountSummary>;
}

export function AddressTable({ items }: AddressTableProps) {
  if (items.length === 0) {
    return (
      <p className="py-8 text-center text-gray-500">No addresses found.</p>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Address</TableHead>
          <TableHead className="text-right">Balance (ART)</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((account) => (
          <TableRow key={account.address}>
            <TableCell>
              <AddressLink address={account.address} head={10} tail={8} />
            </TableCell>
            <TableCell className="text-right tabular-nums">
              {formatBalance(account.balance, 18)}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
