"use client";

import { useEffect, useState } from "react";
import { type Address, type Log } from "viem";
import { usePublicClient } from "wagmi";

export function useContractLogs(address: Address) {
  const [logs, setLogs] = useState<Log[]>([]);
  const client = usePublicClient();

  useEffect(() => {
    if (!client) return;

    async function fetchLogs() {
      const existing = await client!.getLogs({
        address,
        fromBlock: 0n,
        toBlock: "latest",
      });
      setLogs(existing);
    }

    fetchLogs();

    return client.watchBlockNumber({
      onBlockNumber: async (_blockNumber, prevBlockNumber) => {
        const newLogs = await client!.getLogs({
          address,
          fromBlock: prevBlockNumber,
          toBlock: "latest",
        });
        setLogs((prev) => [...prev, ...newLogs]);
      },
    });
  }, [address, client]);

  return logs;
}
