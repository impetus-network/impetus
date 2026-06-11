import { z } from "zod";
import { Decimal } from "@prisma/client/runtime/library";
import { publicProcedure, router } from "../trpc";
import { checksumAddr, safeNum, decStr } from "../lib/encode";
import { encodeCursor, decodeCursor } from "../lib/cursor";

interface AccountCursor {
  id: number;
}

export const addressRouter = router({
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
        ? decodeCursor<AccountCursor>(input.cursor)
        : null;

      const rows = await ctx.prisma.balance_accounts.findMany({
        orderBy: { id: "desc" },
        take: limit + 1,
        ...(decoded
          ? { where: { id: { lt: BigInt(decoded.id) } } }
          : {}),
      });

      const hasMore = rows.length > limit;
      const items = hasMore ? rows.slice(0, limit) : rows;

      const lastItem = items.length > 0 ? items[items.length - 1]! : null;
      const firstItem = items.length > 0 ? items[0]! : null;

      const nextCursor =
        hasMore && lastItem
          ? encodeCursor({ id: safeNum(lastItem.id) })
          : null;

      const prevCursor =
        decoded && firstItem
          ? encodeCursor({ id: safeNum(firstItem.id) + 1 })
          : null;

      return {
        items: items.map((row) => ({
          address: row.address ? checksumAddr(row.address) : "",
          balance: decStr(row.balance ?? new Decimal(0)),
        })),
        nextCursor,
        prevCursor,
      };
    }),

  byAddress: publicProcedure
    .input(z.object({ addr: z.string().regex(/^0x[0-9a-fA-F]{40}$/i) }))
    .query(async ({ ctx, input }) => {
      const normalized = input.addr.toLowerCase();
      const row = await ctx.prisma.balance_accounts.findFirst({
        where: { address: normalized },
      });

      if (!row) return null;

      return {
        address: row.address ? checksumAddr(row.address) : "",
        balance: decStr(row.balance ?? new Decimal(0)),
        nonce: safeNum(row.nonce ?? 0n),
        locked: decStr(row.locked ?? new Decimal(0)),
        reserved: decStr(row.reserved ?? new Decimal(0)),
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
