import { z } from "zod";
import { Decimal } from "@prisma/client/runtime/library";
import { publicProcedure, router } from "../trpc";
import { checksumAddr, safeNum, decStr } from "../lib/encode";
import { encodeCursor, decodeCursor } from "../lib/cursor";

interface ContractCursor {
  transaction_count: number;
  address: string;
}

export const contractRouter = router({
  list: publicProcedure
    .input(
      z.object({
        cursor: z.string().nullish(),
        limit: z.number().int().min(1).max(50).default(20),
      }),
    )
    .query(async ({ ctx, input }) => {
      const { limit } = input;
      const decoded = input.cursor
        ? decodeCursor<ContractCursor>(input.cursor)
        : null;

      const rows = await ctx.prisma.evm_contracts.findMany({
        orderBy: [
          { transaction_count: "desc" },
          { address: "asc" },
        ],
        take: limit + 1,
        ...(decoded
          ? {
              where: {
                OR: [
                  {
                    transaction_count: {
                      lt: BigInt(decoded.transaction_count),
                    },
                  },
                  {
                    transaction_count: BigInt(decoded.transaction_count),
                    address: { gt: decoded.address },
                  },
                ],
              },
            }
          : {}),
      });

      const hasMore = rows.length > limit;
      const items = hasMore ? rows.slice(0, limit) : rows;

      const lastItem = items.length > 0 ? items[items.length - 1]! : null;

      const nextCursor =
        hasMore && lastItem
          ? encodeCursor({
              transaction_count: safeNum(lastItem.transaction_count ?? 0n),
              address: lastItem.address,
            })
          : null;

      const prevCursor =
        decoded && items.length > 0
          ? encodeCursor({
              transaction_count: safeNum(items[0]!.transaction_count ?? 0n),
              address: items[0]!.address,
            })
          : null;

      return {
        items: items.map((row) => ({
          address: checksumAddr(row.address),
          name: row.contract_name ?? null,
          txCount: safeNum(row.transaction_count ?? 0n),
          verified: row.verify_status === "verified",
        })),
        nextCursor,
        prevCursor,
      };
    }),

  byAddress: publicProcedure
    .input(z.object({ addr: z.string().regex(/^0x[0-9a-fA-F]{40}$/i) }))
    .query(async ({ ctx, input }) => {
      const normalized = input.addr.toLowerCase();
      const row = await ctx.prisma.evm_contracts.findUnique({
        where: { address: normalized },
      });

      if (!row) return null;

      return {
        address: checksumAddr(row.address),
        name: row.contract_name ?? null,
        txCount: safeNum(row.transaction_count ?? 0n),
        verified: row.verify_status === "verified",
        deployer: row.deployer ? checksumAddr(row.deployer) : null,
        txHash: row.tx_hash ?? null,
        abi: row.abi ? JSON.stringify(row.abi) : null,
        sourceCode: row.source_code ?? null,
        bytecode: row.creation_bytecode ?? null,
        compilerVersion: row.compiler_version ?? null,
        evmVersion: row.evm_version ?? null,
      };
    }),

  txList: publicProcedure
    .input(
      z.object({
        addr: z.string().regex(/^0x[0-9a-fA-F]{40}$/i),
        cursor: z.string().nullish(),
        limit: z.number().int().min(1).max(50).default(20),
      }),
    )
    .query(async ({ ctx, input }) => {
      const { limit } = input;
      const normalized = input.addr.toLowerCase();
      const decoded = input.cursor
        ? decodeCursor<{ transaction_id: number }>(input.cursor)
        : null;

      const rows = await ctx.prisma.evm_transactions.findMany({
        where: {
          OR: [
            { from_address: normalized },
            { to_address: normalized },
          ],
          ...(decoded
            ? { transaction_id: { lt: BigInt(decoded.transaction_id) } }
            : {}),
        },
        orderBy: { transaction_id: "desc" },
        take: limit + 1,
      });

      const hasMore = rows.length > limit;
      const items = hasMore ? rows.slice(0, limit) : rows;

      const lastItem = items.length > 0 ? items[items.length - 1]! : null;
      const firstItem = items.length > 0 ? items[0]! : null;

      const nextCursor =
        hasMore && lastItem?.transaction_id != null
          ? encodeCursor({
              transaction_id: safeNum(lastItem.transaction_id),
            })
          : null;

      const prevCursor =
        decoded && firstItem?.transaction_id != null
          ? encodeCursor({
              transaction_id: safeNum(firstItem.transaction_id) + 1,
            })
          : null;

      return {
        items: items.map((row) => ({
          hash: row.hash,
          blockNum: safeNum(row.block_num ?? 0n),
          from: row.from_address ? checksumAddr(row.from_address) : "",
          to: row.to_address ? checksumAddr(row.to_address) : null,
          value: decStr(row.value ?? new Decimal(0)),
          timestamp: Number(row.block_timestamp ?? 0n),
        })),
        nextCursor,
        prevCursor,
      };
    }),
});
