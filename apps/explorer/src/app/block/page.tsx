import { getServerCaller } from "@/server/caller";
import { BlockTable } from "@/components/block/block-table";
import { Pagination } from "@/components/shared/pagination";

export default async function BlockListPage({
  searchParams,
}: {
  searchParams: Promise<{ cursor?: string }>;
}) {
  const { cursor } = await searchParams;
  const trpc = await getServerCaller();
  const result = await trpc.block.list({ cursor, limit: 25 });

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">Blocks</h1>
      <BlockTable items={result.items} />
      <Pagination
        nextCursor={result.nextCursor}
        prevCursor={result.prevCursor}
        basePath="/block"
      />
    </main>
  );
}
