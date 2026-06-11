import { notFound } from "next/navigation";
import { getServerCaller } from "@/server/caller";
import { TxInfoPanel } from "@/components/tx/tx-info-panel";

export default async function TxDetailPage({
  params,
}: {
  params: Promise<{ hash: string }>;
}) {
  const { hash } = await params;

  if (!/^0x[0-9a-fA-F]{64}$/.test(hash)) notFound();

  const trpc = await getServerCaller();
  const tx = await trpc.tx.byHash({ hash });
  if (!tx) notFound();

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">Transaction Details</h1>

      <section className="rounded-lg border border-gray-200 bg-white p-6">
        <TxInfoPanel tx={tx} />
      </section>
    </main>
  );
}
