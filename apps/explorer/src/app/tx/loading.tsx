import { Skeleton } from "@artemis/coss-ui/ui/skeleton";

export default function TxLoading() {
  return (
    <main className="mx-auto max-w-7xl px-4 py-8">
      <Skeleton className="mb-4 h-8 w-48" />
      <div className="space-y-2">
        {Array.from({ length: 10 }, (_, i) => (
          <Skeleton key={i} className="h-12 w-full" />
        ))}
      </div>
    </main>
  );
}
