"use client";

import { useQuery } from "@tanstack/react-query";
import { SQUID_URL } from "~/config/indexer";

export interface GaslessRuleRow {
  id: string;
  contract: string;
  selector: string;
  enabled: boolean;
  minValue: string;
  updatedAtBlock: number;
}

// Subsquid OpenReader query shape (entity GaslessRule -> `gaslessRules`).
const RULES_QUERY = `{
  gaslessRules(orderBy: [updatedAtBlock_DESC], limit: 100) {
    id
    contract
    selector
    enabled
    minValue
    updatedAtBlock
  }
}`;

async function fetchRules(): Promise<GaslessRuleRow[]> {
  const res = await fetch(`${SQUID_URL}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: RULES_QUERY }),
  });
  const json = await res.json();
  return json.data?.gaslessRules ?? [];
}

export function useGaslessRules() {
  return useQuery({
    queryKey: ["gaslessRules"],
    queryFn: fetchRules,
    refetchInterval: 5000,
  });
}
