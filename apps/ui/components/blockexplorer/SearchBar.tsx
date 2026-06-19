"use client";

import { type FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { isAddress, isHex } from "viem";

/** Route a query to the right explorer page: address, tx hash, or block number. */
export function resolveSearch(raw: string): string | null {
  const q = raw.trim();
  if (!q) return null;
  if (isAddress(q)) return `/blockexplorer/address/${q}`;
  if (isHex(q) && q.length === 66) return `/blockexplorer/tx/${q}`;
  if (/^\d+$/.test(q)) return `/blockexplorer/block/${q}`;
  return null;
}

export function SearchBar() {
  const [query, setQuery] = useState("");
  const router = useRouter();

  function handleSearch(e: FormEvent) {
    e.preventDefault();
    const target = resolveSearch(query);
    if (target) {
      router.push(target);
      setQuery("");
    }
  }

  return (
    <form onSubmit={handleSearch} className="flex gap-2">
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search by address, tx hash, or block number…"
        className="flex-1 rounded-lg border border-border bg-background px-4 py-2 text-sm focus:border-primary focus:outline-none"
      />
      <button
        type="submit"
        className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:opacity-90"
      >
        Search
      </button>
    </form>
  );
}
