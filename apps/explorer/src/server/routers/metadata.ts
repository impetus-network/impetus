import { publicProcedure, router } from "../trpc";
import { env } from "@/env";

export const metadataRouter = router({
  get: publicProcedure.query(async ({ ctx }) => {
    const [blockAgg, txCount, accountCount, contractCount] =
      await Promise.all([
        ctx.prisma.evm_blocks.aggregate({ _max: { block_num: true } }),
        ctx.prisma.evm_transactions.count(),
        ctx.prisma.balance_accounts.count(),
        ctx.prisma.evm_contracts.count(),
      ]);

    const blockHeight = Number(blockAgg._max.block_num ?? 0n);

    return {
      chainId: env.NEXT_PUBLIC_CHAIN_ID,
      chainName: env.NEXT_PUBLIC_CHAIN_NAME,
      tokenSymbol: env.NEXT_PUBLIC_TOKEN_SYMBOL,
      tokenDecimals: env.NEXT_PUBLIC_TOKEN_DECIMALS,
      blockHeight,
      finalizedHeight: blockHeight,
      txCount,
      accountCount,
      contractCount,
    };
  }),
});
