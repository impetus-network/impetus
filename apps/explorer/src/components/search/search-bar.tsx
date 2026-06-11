"use client";

import { useRef } from "react";
import { searchAction } from "@/app/search/actions";

export function SearchBar() {
  const formRef = useRef<HTMLFormElement>(null);

  return (
    <form ref={formRef} action={searchAction} className="flex w-full max-w-md">
      <input
        name="query"
        type="text"
        placeholder="Search by block, tx hash, or address..."
        className="flex-1 rounded-l-lg border border-gray-300 px-3 py-2 text-sm outline-none focus:border-gray-500"
      />
      <button
        type="submit"
        className="rounded-r-lg bg-gray-900 px-4 py-2 text-sm text-white hover:bg-gray-800"
      >
        Search
      </button>
    </form>
  );
}
