"use server";

import { redirect } from "next/navigation";
import { getServerCaller } from "@/server/caller";

export async function searchAction(formData: FormData) {
  const query = formData.get("query");
  if (typeof query !== "string" || !query.trim()) return;

  const trpc = await getServerCaller();
  const result = await trpc.search.resolve({ query: query.trim() });

  switch (result.kind) {
    case "block":
      redirect(`/block/${result.blockNum}`);
    case "tx":
      redirect(`/tx/${result.hash}`);
    case "address":
      redirect(`/address/${result.address}`);
    case "contract":
      redirect(`/contract/${result.address}`);
    case "not_found":
      redirect(`/?error=not-found`);
  }
}
