"use client";

import { type Abi, type Log } from "viem";
import { useWatchContractEvent } from "wagmi";
import { debugContracts } from "~/config/contracts";

type ContractName = (typeof debugContracts)[number]["name"];

interface UseScaffoldWatchContractEventParams {
  contractName: ContractName;
  eventName: string;
  onLogs: (logs: Log[]) => void;
  enabled?: boolean;
}

export function useScaffoldWatchContractEvent({
  contractName,
  eventName,
  onLogs,
  enabled = true,
}: UseScaffoldWatchContractEventParams) {
  const contract = debugContracts.find((c) => c.name === contractName);

  return useWatchContractEvent({
    address: contract?.address,
    abi: (contract?.abi ?? []) as Abi,
    eventName,
    onLogs,
    enabled: enabled && !!contract,
  });
}
