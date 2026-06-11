import { getServerCaller } from "@/server/caller";
import { ContractTable } from "@/components/contract/contract-table";
import { Pagination } from "@/components/shared/pagination";

export default async function ContractListPage({
  searchParams,
}: {
  searchParams: Promise<{ cursor?: string }>;
}) {
  const { cursor } = await searchParams;
  const trpc = await getServerCaller();
  const result = await trpc.contract.list({ cursor, limit: 25 });

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">Contracts</h1>
      <ContractTable items={result.items} />
      <Pagination
        nextCursor={result.nextCursor}
        prevCursor={result.prevCursor}
        basePath="/contract"
      />
    </main>
  );
}
