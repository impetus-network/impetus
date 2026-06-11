"use client";

import { type Abi, type AbiEvent, type Log } from "viem";
import { usePublicClient, useBlockNumber } from "wagmi";
import { useQuery } from "@tanstack/react-query";
import { debugContracts } from "~/config/contracts";

type ContractName = (typeof debugContracts)[number]["name"];

interface UseScaffoldEventHistoryParams {
  contractName: ContractName;
  eventName: string;
  fromBlock?: bigint;
  enabled?: boolean;
  watch?: boolean;
}

export function useScaffoldEventHistory({
  contractName,
  eventName,
  fromBlock = 0n,
  enabled = true,
  watch = false,
}: UseScaffoldEventHistoryParams) {
  const client = usePublicClient();
  const contract = debugContracts.find((c) => c.name === contractName);
  const { data: blockNumber } = useBlockNumber({ watch });

  const event = contract
    ? (contract.abi as Abi).find(
        (item): item is AbiEvent => item.type === "event" && "name" in item && item.name === eventName,
      )
    : undefined;

  return useQuery({
    queryKey: ["scaffoldEventHistory", contractName, eventName, fromBlock.toString(), blockNumber?.toString()],
    queryFn: async () => {
      if (!client || !contract || !event) return [];

      const logs = await client.getLogs({
        address: contract.address,
        event,
        fromBlock,
        toBlock: "latest",
      });

      return logs.reverse() as Log[];
    },
    enabled: enabled && !!client && !!contract && !!event,
  });
}
