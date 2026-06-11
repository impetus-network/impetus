"use client";

import { BlockieAvatar } from "./BlockieAvatar";
import { useCopyToClipboard } from "~/hooks/useCopyToClipboard";

interface AddressProps {
  address: string;
  format?: "short" | "full";
}

export function Address({ address, format = "short" }: AddressProps) {
  const { copied, copy } = useCopyToClipboard();
  const display = format === "short" ? `${address.slice(0, 6)}...${address.slice(-4)}` : address;

  return (
    <button
      onClick={() => copy(address)}
      className="inline-flex items-center gap-1.5 rounded px-1 py-0.5 font-mono text-sm hover:bg-muted"
      title={copied ? "Copied!" : "Click to copy"}
    >
      <BlockieAvatar address={address} size={18} />
      <span>{display}</span>
      {copied && <span className="text-xs text-success">✓</span>}
    </button>
  );
}
