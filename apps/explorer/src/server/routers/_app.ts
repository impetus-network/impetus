import { router } from "../trpc";
import { metadataRouter } from "./metadata";
import { blockRouter } from "./block";
import { txRouter } from "./tx";
import { addressRouter } from "./address";
import { contractRouter } from "./contract";
import { searchRouter } from "./search";

export const appRouter = router({
  metadata: metadataRouter,
  block: blockRouter,
  tx: txRouter,
  address: addressRouter,
  contract: contractRouter,
  search: searchRouter,
});

export type AppRouter = typeof appRouter;
