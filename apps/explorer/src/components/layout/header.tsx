import Link from "next/link";
import { Nav } from "./nav";
import { SearchBar } from "@/components/search/search-bar";

export function Header() {
  return (
    <header className="border-b border-gray-200 bg-white">
      <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3">
        <Link
          href="/"
          className="text-lg font-bold text-gray-900 tracking-tight"
        >
          Artemis Explorer
        </Link>
        <div className="flex items-center gap-6">
          <Nav />
          <div className="hidden sm:block">
            <SearchBar />
          </div>
        </div>
      </div>
    </header>
  );
}
