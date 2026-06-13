"use client";

import { type Address, formatEther } from "viem";
import { useAccount } from "wagmi";
import { useScaffoldReadContract } from "./useScaffoldReadContract";

export interface StakingStatus {
  isBonded: boolean;
  activeWei: bigint;
  totalWei: bigint;
  active: number;
  total: number;
  nominating: readonly Address[];
  isNominating: boolean;
  isLoading: boolean;
  refetch: () => void;
}

// Reads the connected account's staking ledger + nomination targets.
// A never-bonded account makes `ledger`/`nominators` revert; viem surfaces that
// as an error with `data === undefined`, which we treat as "not bonded".
export function useStakingStatus(): StakingStatus {
  const { address } = useAccount();
  const enabled = !!address;

  const ledger = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "ledger",
    args: address ? [address] : undefined,
    enabled,
  });
  const nominators = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "nominators",
    args: address ? [address] : undefined,
    enabled,
  });

  const ledgerData = ledger.data as readonly [bigint, bigint, unknown] | undefined;
  const activeWei = ledgerData?.[0] ?? 0n;
  const totalWei = ledgerData?.[1] ?? 0n;

  const nomData = nominators.data as
    | readonly [readonly Address[], number, boolean]
    | undefined;
  const nominating = nomData?.[0] ?? [];

  return {
    isBonded: totalWei > 0n,
    activeWei,
    totalWei,
    active: Number(formatEther(activeWei)),
    total: Number(formatEther(totalWei)),
    nominating,
    isNominating: nominating.length > 0,
    isLoading: ledger.isLoading || nominators.isLoading,
    refetch: () => {
      void ledger.refetch();
      void nominators.refetch();
    },
  };
}
