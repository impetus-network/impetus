"use client";

import { type FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import { isAddress, isHex } from "viem";

export function SearchBar() {
  const [query, setQuery] = useState("");
  const router = useRouter();

  function handleSearch(e: FormEvent) {
    e.preventDefault();
    const trimmed = query.trim();
    if (isAddress(trimmed)) {
      router.push(`/blockexplorer/address/${trimmed}`);
    } else if (isHex(trimmed) && trimmed.length === 66) {
      router.push(`/blockexplorer/tx/${trimmed}`);
    }
    setQuery("");
  }

  return (
    <form onSubmit={handleSearch} className="flex gap-2">
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search by address or tx hash..."
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
