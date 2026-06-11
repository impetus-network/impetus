import { notFound } from "next/navigation";
import { getServerCaller } from "@/server/caller";
import { ContractInfoPanel } from "@/components/contract/contract-info-panel";
import { TxTable } from "@/components/tx/tx-table";
import { Pagination } from "@/components/shared/pagination";

export default async function ContractDetailPage({
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
  const contract = await trpc.contract.byAddress({ addr });
  if (!contract) notFound();

  const txResult = await trpc.contract.txList({
    addr,
    cursor,
    limit: 25,
  });

  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">
        Contract{contract.name ? `: ${contract.name}` : ""}
      </h1>

      <section className="rounded-lg border border-gray-200 bg-white p-6">
        <ContractInfoPanel contract={contract} />
      </section>

      {contract.verified && contract.sourceCode && (
        <section className="mt-8">
          <h2 className="text-lg font-semibold mb-4">Source Code</h2>
          <div className="max-h-96 overflow-auto rounded-lg border border-gray-200 bg-gray-50 p-4">
            <pre className="font-mono text-xs whitespace-pre-wrap">
              {contract.sourceCode}
            </pre>
          </div>
        </section>
      )}

      <section className="mt-8">
        <h2 className="text-lg font-semibold mb-4">Transactions</h2>
        <TxTable items={txResult.items} />
        <Pagination
          nextCursor={txResult.nextCursor}
          prevCursor={txResult.prevCursor}
          basePath={`/contract/${addr}`}
        />
      </section>
    </main>
  );
}
