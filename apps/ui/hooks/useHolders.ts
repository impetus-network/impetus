"use client";

import { useQuery } from "@tanstack/react-query";
import { SQUID_URL } from "~/config/indexer";

export interface HolderRow {
  id: string; // address (0x…)
  free: string;
  reserved: string;
  total: string;
  updatedAt: number;
}

export interface HoldersData {
  holders: HolderRow[];
  totalIssuance: string;
  holdersCount: number;
}

// Subsquid OpenReader: native holders ranked by total balance, plus the
// ChainStat singleton for supply + count. Balance is authoritative
// (System.Account storage), so no client-side derivation is needed.
const HOLDERS_QUERY = `query Holders($limit: Int!) {
  holders(orderBy: [total_DESC], where: { total_gt: "0" }, limit: $limit) {
    id
    free
    reserved
    total
    updatedAt
  }
  chainStats(limit: 1) {
    totalIssuance
    holdersCount
  }
}`;

async function fetchHolders(limit: number): Promise<HoldersData> {
  const res = await fetch(`${SQUID_URL}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: HOLDERS_QUERY, variables: { limit } }),
  });
  const json = await res.json();
  const stat = json.data?.chainStats?.[0];
  return {
    holders: json.data?.holders ?? [],
    totalIssuance: stat?.totalIssuance ?? "0",
    holdersCount: stat?.holdersCount ?? 0,
  };
}

export function useHolders(limit = 50) {
  return useQuery({
    queryKey: ["holders", limit],
    queryFn: () => fetchHolders(limit),
    refetchInterval: 12000,
  });
}
