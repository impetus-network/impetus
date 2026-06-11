import { notFound } from "next/navigation";
import { getServerCaller } from "@/server/caller";
import { AddressInfoPanel } from "@/components/address/address-info-panel";
import { TxTable } from "@/components/tx/tx-table";
import { Pagination } from "@/components/shared/pagination";

export default async function AddressDetailPage({
  params,
  searchParams,
}: {
  params: Promise<{ addr: string }>;
  searchParams: Promise<{ cursor?: string }>;
}) {
  const { addr } = await params;
  const { cursor } = await searchParams;

  if (!/^0x[0-9a-fA-F]{40}$/i.test(addr)) notFound();

  const trpc = await getServerCaller();
  const account = await trpc.address.byAddress({ addr });
  if (!account) notFound();

  const txResult = await trpc.address.txList({
    addr,
    cursor,
    limit: 25,
  });

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">Address</h1>

      <section className="rounded-lg border border-gray-200 bg-white p-6">
        <AddressInfoPanel account={account} />
      </section>

      <section className="mt-8">
        <h2 className="text-lg font-semibold mb-4">Transactions</h2>
        <TxTable items={txResult.items} />
        <Pagination
          nextCursor={txResult.nextCursor}
          prevCursor={txResult.prevCursor}
          basePath={`/address/${addr}`}
        />
      </section>
    </main>
  );
}
