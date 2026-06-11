export type SearchCategory = "block_num" | "tx_hash" | "address" | "unknown";

/** Classify a search query into a category for routing. */
export function classify(q: string): SearchCategory {
  const trimmed = q.trim();
  if (/^\d+$/.test(trimmed)) return "block_num";
  if (/^0x[0-9a-fA-F]{64}$/.test(trimmed)) return "tx_hash";
  if (/^0x[0-9a-fA-F]{40}$/.test(trimmed)) return "address";
  return "unknown";
}
