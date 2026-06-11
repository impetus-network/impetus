import { notFound } from "next/navigation";
import { getServerCaller } from "@/server/caller";
import { BlockInfoPanel } from "@/components/block/block-info-panel";
import { TxTable } from "@/components/tx/tx-table";
import { Pagination } from "@/components/shared/pagination";

export default async function BlockDetailPage({
  params,
  searchParams,
}: {
  params: Promise<{ num: string }>;
  searchParams: Promise<{ cursor?: string }>;
}) {
  const { num } = await params;
  const { cursor } = await searchParams;
  const blockNum = parseInt(num, 10);

  if (Number.isNaN(blockNum) || blockNum < 0) notFound();

  const trpc = await getServerCaller();
  const block = await trpc.block.byNumber({ num: blockNum });
  if (!block) notFound();

  const txResult = await trpc.block.txList({
    blockNum,
    cursor,
    limit: 25,
  });

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">Block #{num}</h1>

      <section className="rounded-lg border border-gray-200 bg-white p-6">
        <BlockInfoPanel block={block} />
      </section>

      <section className="mt-8">
        <h2 className="text-lg font-semibold mb-4">Transactions</h2>
        <TxTable items={txResult.items} />
        <Pagination
          nextCursor={txResult.nextCursor}
          prevCursor={txResult.prevCursor}
          basePath={`/block/${num}`}
        />
      </section>
    </main>
  );
}
