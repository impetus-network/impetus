import { z } from "zod";
import { Decimal } from "@prisma/client/runtime/library";
import { publicProcedure, router } from "../trpc";
import { checksumAddr, safeNum, decStr } from "../lib/encode";
import { encodeCursor, decodeCursor } from "../lib/cursor";

interface TxCursor {
  transaction_id: number;
}

function toTxSummary(row: {
  hash: string;
  block_num: bigint | null;
  from_address: string | null;
  to_address: string | null;
  value: Decimal | null;
  block_timestamp: bigint | null;
}) {
  return {
    hash: row.hash,
    blockNum: safeNum(row.block_num ?? 0n),
    from: row.from_address ? checksumAddr(row.from_address) : "",
    to: row.to_address ? checksumAddr(row.to_address) : null,
    value: decStr(row.value ?? new Decimal(0)),
    timestamp: Number(row.block_timestamp ?? 0n),
  };
}

function computeFee(
  gasUsed: Decimal | null,
  effectiveGasPrice: Decimal | null,
): string {
  const used = BigInt(decStr(gasUsed ?? new Decimal(0)));
  const price = BigInt(decStr(effectiveGasPrice ?? new Decimal(0)));
  return (used * price).toString();
}

export const txRouter = router({
  latest: publicProcedure
    .input(z.object({ limit: z.number().int().min(1).max(50).default(10) }))
    .query(async ({ ctx, input }) => {
      const rows = await ctx.prisma.evm_transactions.findMany({
        orderBy: { transaction_id: "desc" },
        take: input.limit,
      });
      return rows.map(toTxSummary);
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
        ? decodeCursor<TxCursor>(input.cursor)
        : null;

      const rows = await ctx.prisma.evm_transactions.findMany({
        orderBy: { transaction_id: "desc" },
        take: limit + 1,
        ...(decoded
          ? {
              where: {
                transaction_id: { lt: BigInt(decoded.transaction_id) },
              },
            }
          : {}),
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
        items: items.map(toTxSummary),
        nextCursor,
        prevCursor,
      };
    }),

  byHash: publicProcedure
    .input(z.object({ hash: z.string().regex(/^0x[0-9a-fA-F]{64}$/) }))
    .query(async ({ ctx, input }) => {
      const row = await ctx.prisma.evm_transactions.findUnique({
        where: { hash: input.hash },
      });

      if (!row) return null;

      return {
        ...toTxSummary(row),
        status: row.success ? ("Success" as const) : ("Failed" as const),
        nonce: safeNum(row.nonce ?? 0n),
        inputData: row.input_data ?? "",
        fee: computeFee(row.gas_used, row.effective_gas_price),
        gasUsed: decStr(row.gas_used ?? new Decimal(0)),
        gasPrice: decStr(row.gas_price ?? new Decimal(0)),
      };
    }),
});
