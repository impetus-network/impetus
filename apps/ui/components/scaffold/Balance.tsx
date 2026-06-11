"use client";

import { useBalance } from "wagmi";
import { formatEther } from "viem";
import type { Address as AddressType } from "viem";

interface BalanceProps {
  address: AddressType;
}

export function Balance({ address }: BalanceProps) {
  const { data, isLoading } = useBalance({ address });

  if (isLoading) return <span className="text-muted-foreground">...</span>;
  if (!data) return <span className="text-muted-foreground">—</span>;

  return (
    <span className="font-mono">
      {Number(formatEther(data.value)).toFixed(4)} {data.symbol}
    </span>
  );
}
