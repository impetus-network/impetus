"use client";

import { useEffect, useState } from "react";
import { usePublicClient, useBlockNumber } from "wagmi";
import type { Block, Transaction } from "viem";

export function useFetchBlocks(count = 10) {
  const publicClient = usePublicClient();
  const { data: latestBlock } = useBlockNumber({ watch: true });
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!publicClient || latestBlock === undefined) return;

    async function fetchBlocks() {
      setLoading(true);
      const blockNumbers = Array.from(
        { length: Math.min(count, Number(latestBlock!) + 1) },
        (_, i) => latestBlock! - BigInt(i),
      );

      const fetchedBlocks = await Promise.all(
        blockNumbers.map((n) => publicClient!.getBlock({ blockNumber: n, includeTransactions: true })),
      );

      setBlocks(fetchedBlocks);

      const allTxs = fetchedBlocks.flatMap((b) => (b.transactions || []) as unknown as Transaction[]);
      setTransactions(allTxs.slice(0, 20));
      setLoading(false);
    }

    fetchBlocks();
  }, [publicClient, latestBlock, count]);

  return { blocks, transactions, loading, latestBlock };
}
