import { z } from "zod";
import { Decimal } from "@prisma/client/runtime/library";
import { publicProcedure, router } from "../trpc";
import { checksumAddr, safeNum, decStr } from "../lib/encode";
import { encodeCursor, decodeCursor } from "../lib/cursor";

interface BlockCursor {
  block_num: number;
}

function toBlockSummary(row: {
  block_num: bigint;
  block_hash: string | null;
  miner: string | null;
  timestamp: bigint | null;
  transaction_count: number | null;
}) {
  return {
    blockNum: safeNum(row.block_num),
    hash: row.block_hash ?? "",
    miner: row.miner ? checksumAddr(row.miner) : "",
    timestamp: Number(row.timestamp ?? 0n),
    txCount: row.transaction_count ?? 0,
  };
}

function toBlockDetail(row: {
  block_num: bigint;
  block_hash: string | null;
  parent_hash: string | null;
  miner: string | null;
  timestamp: bigint | null;
  transaction_count: number | null;
  gas_used: Decimal | null;
  gas_limit: Decimal | null;
  block_size: Decimal | null;
}) {
  return {
    ...toBlockSummary(row),
    parentHash: row.parent_hash ?? "",
    gasUsed: decStr(row.gas_used ?? new Decimal(0)),
    gasLimit: decStr(row.gas_limit ?? new Decimal(0)),
    size: decStr(row.block_size ?? new Decimal(0)),
  };
}

export const blockRouter = router({
  latest: publicProcedure
    .input(z.object({ limit: z.number().int().min(1).max(50).default(10) }))
    .query(async ({ ctx, input }) => {
      const rows = await ctx.prisma.evm_blocks.findMany({
        orderBy: { block_num: "desc" },
        take: input.limit,
      });
      return rows.map(toBlockSummary);
    }),

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
        ? decodeCursor<BlockCursor>(input.cursor)
        : null;

      const rows = await ctx.prisma.evm_blocks.findMany({
        orderBy: { block_num: "desc" },
        take: limit + 1,
        ...(decoded
          ? { where: { block_num: { lt: BigInt(decoded.block_num) } } }
          : {}),
      });

      const hasMore = rows.length > limit;
      const items = hasMore ? rows.slice(0, limit) : rows;

      const nextCursor =
        hasMore && items.length > 0
          ? encodeCursor({
              block_num: safeNum(items[items.length - 1]!.block_num),
            })
          : null;

      const prevCursor =
        decoded && items.length > 0
          ? encodeCursor({ block_num: safeNum(items[0]!.block_num) + 1 })
          : null;

      return {
        items: items.map(toBlockSummary),
        nextCursor,
        prevCursor,
      };
    }),

  byNumber: publicProcedure
    .input(z.object({ num: z.number().int().min(0) }))
    .query(async ({ ctx, input }) => {
      const row = await ctx.prisma.evm_blocks.findUnique({
        where: { block_num: BigInt(input.num) },
      });
      return row ? toBlockDetail(row) : null;
    }),

  txList: publicProcedure
    .input(
      z.object({
        blockNum: z.number().int().min(0),
        cursor: z.string().nullish(),
        limit: z.number().int().min(1).max(50).default(20),
      }),
    )
    .query(async ({ ctx, input }) => {
      const { limit, blockNum } = input;
      const decoded = input.cursor
        ? decodeCursor<{ transaction_id: number }>(input.cursor)
        : null;

      const rows = await ctx.prisma.evm_transactions.findMany({
        where: {
          block_num: BigInt(blockNum),
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
