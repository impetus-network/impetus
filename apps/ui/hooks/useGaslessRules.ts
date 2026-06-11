"use client";

import { useQuery } from "@tanstack/react-query";
import { PONDER_URL } from "~/config/ponder";

export interface GaslessRuleRow {
  id: string;
  contract: string;
  selector: string;
  enabled: boolean;
  minValue: string;
  updatedAtBlock: string;
}

const RULES_QUERY = `{
  gaslessRuless(orderBy: "updatedAtBlock", orderDirection: "desc", limit: 100) {
    items {
      id
      contract
      selector
      enabled
      minValue
      updatedAtBlock
    }
  }
}`;

async function fetchRules(): Promise<GaslessRuleRow[]> {
  const res = await fetch(`${PONDER_URL}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: RULES_QUERY }),
  });
  const json = await res.json();
  return json.data?.gaslessRuless?.items ?? [];
}

export function useGaslessRules() {
  return useQuery({
    queryKey: ["gaslessRules"],
    queryFn: fetchRules,
    refetchInterval: 5000,
  });
}
