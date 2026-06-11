import { cacheLife } from "next/cache";
import { getServerCaller } from "@/server/caller";
import { SummaryCards } from "@/components/home/summary-cards";
import { LatestBlocks } from "@/components/home/latest-blocks";
import { LatestTxs } from "@/components/home/latest-txs";

async function getMetadata() {
  "use cache";
  cacheLife({ revalidate: 30, stale: 0 });
  const trpc = await getServerCaller();
  return trpc.metadata.get();
}

export default async function HomePage() {
  const metadata = await getMetadata();

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-3xl font-bold">Artemis Explorer</h1>
      <p className="mt-2 text-gray-600">
        EVM block explorer for chain ID {metadata.chainId}
      </p>

      <section className="mt-8">
        <SummaryCards metadata={metadata} />
      </section>

      <div className="mt-8 grid gap-6 lg:grid-cols-2">
        <div className="rounded-lg border border-gray-200 bg-white p-6">
          <h2 className="mb-4 text-lg font-semibold text-gray-900">
            Latest Blocks
          </h2>
          <LatestBlocks />
        </div>

        <div className="rounded-lg border border-gray-200 bg-white p-6">
          <h2 className="mb-4 text-lg font-semibold text-gray-900">
            Latest Transactions
          </h2>
          <LatestTxs />
        </div>
      </div>
    </main>
  );
}
