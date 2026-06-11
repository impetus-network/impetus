import Link from "next/link";
import { formatHex } from "@artemis/shared";

interface AddressLinkProps {
  address: string;
  head?: number;
  tail?: number;
}

export function AddressLink({
  address,
  head = 6,
  tail = 4,
}: AddressLinkProps) {
  const display = formatHex(address as `0x${string}`, head, tail);
  return (
    <Link
      href={`/address/${address}`}
      className="font-mono text-blue-600 hover:text-blue-800 hover:underline"
      title={address}
    >
      {display}
    </Link>
  );
}
