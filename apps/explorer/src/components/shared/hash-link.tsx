import Link from "next/link";
import { formatHex } from "@artemis/shared";

interface HashLinkProps {
  hash: string;
  href: string;
  head?: number;
  tail?: number;
}

export function HashLink({ hash, href, head = 6, tail = 4 }: HashLinkProps) {
  const display = formatHex(hash as `0x${string}`, head, tail);
  return (
    <Link
      href={href}
      className="font-mono text-blue-600 hover:text-blue-800 hover:underline"
      title={hash}
    >
      {display}
    </Link>
  );
}
