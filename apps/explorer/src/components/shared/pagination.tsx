import Link from "next/link";

interface PaginationProps {
  nextCursor: string | null;
  prevCursor: string | null;
  basePath?: string;
}

export function Pagination({
  nextCursor,
  prevCursor,
  basePath = "",
}: PaginationProps) {
  return (
    <nav
      className="mt-6 flex items-center justify-between"
      aria-label="Pagination"
    >
      {prevCursor ? (
        <Link
          href={`${basePath}?cursor=${prevCursor}`}
          className="inline-flex items-center gap-1.5 rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 transition-colors"
        >
          Previous
        </Link>
      ) : (
        <span className="inline-flex items-center gap-1.5 rounded-md border border-gray-200 px-4 py-2 text-sm font-medium text-gray-400 cursor-not-allowed">
          Previous
        </span>
      )}
      {nextCursor ? (
        <Link
          href={`${basePath}?cursor=${nextCursor}`}
          className="inline-flex items-center gap-1.5 rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 transition-colors"
        >
          Next
        </Link>
      ) : (
        <span className="inline-flex items-center gap-1.5 rounded-md border border-gray-200 px-4 py-2 text-sm font-medium text-gray-400 cursor-not-allowed">
          Next
        </span>
      )}
    </nav>
  );
}
