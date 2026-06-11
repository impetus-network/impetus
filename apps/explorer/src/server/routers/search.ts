import { z } from "zod";
import { publicProcedure, router } from "../trpc";
import { classify } from "@/lib/regex";
import { checksumAddr } from "../lib/encode";

type SearchResult =
  | { kind: "block"; blockNum: number }
  | { kind: "tx"; hash: string }
  | { kind: "address"; address: string }
  | { kind: "contract"; address: string }
  | { kind: "not_found" };

export const searchRouter = router({
  resolve: publicProcedure
    .input(z.object({ query: z.string().min(1).max(256) }))
    .query(async ({ ctx, input }): Promise<SearchResult> => {
      const q = input.query.trim();
      const category = classify(q);

      switch (category) {
        case "block_num": {
          const row = await ctx.prisma.evm_blocks.findUnique({
            where: { block_num: BigInt(q) },
            select: { block_num: true },
          });
          return row
            ? { kind: "block", blockNum: Number(row.block_num) }
            : { kind: "not_found" };
        }

        case "tx_hash": {
          const row = await ctx.prisma.evm_transactions.findUnique({
            where: { hash: q },
            select: { hash: true },
          });
          return row
            ? { kind: "tx", hash: row.hash }
            : { kind: "not_found" };
        }

        case "address": {
          const normalized = q.toLowerCase();

          // Check contracts first
          const contract = await ctx.prisma.evm_contracts.findUnique({
            where: { address: normalized },
            select: { address: true },
          });
          if (contract) {
            return { kind: "contract", address: checksumAddr(contract.address) };
          }

          // Check balance_accounts
          const account = await ctx.prisma.balance_accounts.findFirst({
            where: { address: normalized },
            select: { address: true },
          });
          if (account?.address) {
            return { kind: "address", address: checksumAddr(account.address) };
          }

          return { kind: "not_found" };
        }

        default:
          return { kind: "not_found" };
      }
    }),
});
