// Raw-SQL writer for the four Subscan-shape explorer tables. Runs alongside the
// Squid TypeORM staking/holders store in the SAME Postgres (impetus_squid), via
// its own node-postgres pool. The Squid TypeORM store and this writer commit in
// separate transactions; on a crash between them Squid re-delivers the batch and
// every write here is idempotent, so they re-converge.
//
// Reorg safety: this writer is OUTSIDE Squid's hot-block rollback. We make it
// reorg-safe by (a) deterministic keys (deterministic transaction_id, natural
// @id upserts) and (b) a per-batch DELETE of everything at/after the batch's
// first height before re-inserting -- because Squid re-delivers from the fork
// point on a reorg, the orphaned tip is deleted and the canonical branch
// re-inserted.

import { Pool, type PoolClient } from "pg";
import * as fs from "fs";
import * as path from "path";
import type { EvmBatchRows } from "./evm-build";

export interface BalanceRow {
  address: string;
  balance: string; // free
  locked: string; // frozen
  reserved: string;
  nonce: string;
}

const DDL_PATH = path.join(
  __dirname,
  "..",
  "db",
  "subscan",
  "0001-subscan-tables.sql",
);

const INSERT_CHUNK = 500;

function chunk<T>(arr: readonly T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < arr.length; i += size) out.push(arr.slice(i, i + size));
  return out;
}

/** Chunked multi-row INSERT. `tail` is an optional ON CONFLICT clause. */
async function insertRows(
  client: PoolClient,
  table: string,
  columns: readonly string[],
  rows: readonly unknown[][],
  tail = "",
): Promise<void> {
  const cols = columns.join(", ");
  for (const part of chunk(rows, INSERT_CHUNK)) {
    const placeholders = part
      .map(
        (row, i) =>
          "(" +
          row.map((_, j) => `$${i * columns.length + j + 1}`).join(", ") +
          ")",
      )
      .join(", ");
    const flat = part.flat();
    await client.query(
      `INSERT INTO ${table} (${cols}) VALUES ${placeholders}${tail}`,
      flat,
    );
  }
}

const BLOCK_COLUMNS = [
  "block_num",
  "block_hash",
  "parent_hash",
  "sha3_uncles",
  "author",
  "miner",
  "state_root",
  "transactions_root",
  "receipts_root",
  "gas_used",
  "gas_limit",
  "extra_data",
  "logs_bloom",
  "timestamp",
  "block_size",
  "transaction_count",
  "base_fee_per_gas",
] as const;

const TX_COLUMNS = [
  "hash",
  "block_num",
  "block_timestamp",
  "from_address",
  "to_address",
  "input_data",
  "nonce",
  "gas_limit",
  "gas_price",
  "gas_used",
  "contract",
  "success",
  "r",
  "s",
  "v",
  "value",
  "effective_gas_price",
  "max_priority_fee_per_gas",
  "max_fee_per_gas",
  "cumulative_gas_used",
  "txn_type",
  "transaction_index",
  "transaction_id",
] as const;

const CONTRACT_COLUMNS = [
  "address",
  "deployer",
  "block_num",
  "tx_hash",
  "deploy_at",
  "creation_bytecode",
  "deploy_code_hash",
  "transaction_count",
] as const;

const BALANCE_COLUMNS = [
  "address",
  "balance",
  "locked",
  "reserved",
  "nonce",
] as const;

export interface EvmWriter {
  init(): Promise<void>;
  writeBatch(firstHeight: number, rows: EvmBatchRows): Promise<void>;
  upsertBalances(rows: readonly BalanceRow[]): Promise<void>;
  close(): Promise<void>;
}

export function createEvmWriter(): EvmWriter {
  const pool = new Pool({
    host: process.env.DB_HOST ?? "localhost",
    port: Number(process.env.DB_PORT ?? 5432),
    database: process.env.DB_NAME ?? "postgres",
    user: process.env.DB_USER ?? "postgres",
    password: process.env.DB_PASS ?? "postgres",
    max: 4,
  });
  let initPromise: Promise<void> | undefined;

  async function init(): Promise<void> {
    // Cache the in-flight promise so concurrent callers share one DDL run
    // (the DDL is idempotent regardless, but this avoids redundant work).
    if (!initPromise) {
      initPromise = (async () => {
        const ddl = await fs.promises.readFile(DDL_PATH, "utf8");
        await pool.query(ddl);
      })();
    }
    return initPromise;
  }

  async function writeBatch(
    firstHeight: number,
    rows: EvmBatchRows,
  ): Promise<void> {
    const client = await pool.connect();
    try {
      await client.query("BEGIN");

      await client.query(
        "DELETE FROM evm_transactions WHERE block_num >= $1",
        [firstHeight],
      );
      await client.query("DELETE FROM evm_blocks WHERE block_num >= $1", [
        firstHeight,
      ]);
      await client.query("DELETE FROM evm_contracts WHERE block_num >= $1", [
        firstHeight,
      ]);

      await insertRows(
        client,
        "evm_blocks",
        BLOCK_COLUMNS,
        rows.blocks.map((b) => [
          b.block_num,
          b.block_hash,
          b.parent_hash,
          b.sha3_uncles,
          b.author,
          b.miner,
          b.state_root,
          b.transactions_root,
          b.receipts_root,
          b.gas_used,
          b.gas_limit,
          b.extra_data,
          b.logs_bloom,
          b.timestamp,
          b.block_size,
          b.transaction_count,
          b.base_fee_per_gas,
        ]),
      );

      await insertRows(
        client,
        "evm_transactions",
        TX_COLUMNS,
        rows.txs.map((t) => [
          t.hash,
          t.block_num,
          t.block_timestamp,
          t.from_address,
          t.to_address,
          t.input_data,
          t.nonce,
          t.gas_limit,
          t.gas_price,
          t.gas_used,
          t.contract,
          t.success,
          t.r,
          t.s,
          t.v,
          t.value,
          t.effective_gas_price,
          t.max_priority_fee_per_gas,
          t.max_fee_per_gas,
          t.cumulative_gas_used,
          t.txn_type,
          t.transaction_index,
          t.transaction_id,
        ]),
      );

      if (rows.contracts.length > 0) {
        await insertRows(
          client,
          "evm_contracts",
          CONTRACT_COLUMNS,
          rows.contracts.map((c) => [
            c.address,
            c.deployer,
            c.block_num,
            c.tx_hash,
            c.deploy_at,
            c.creation_bytecode,
            "",
            0,
          ]),
          " ON CONFLICT (address) DO UPDATE SET" +
            " deployer = EXCLUDED.deployer," +
            " block_num = EXCLUDED.block_num," +
            " tx_hash = EXCLUDED.tx_hash," +
            " deploy_at = EXCLUDED.deploy_at," +
            " creation_bytecode = EXCLUDED.creation_bytecode",
        );
      }

      // Maintain evm_contracts.transaction_count (backs the contract.list
      // compound keyset) for every contract whose tx-count could have changed
      // this batch: newly created contracts + any address that appears as a
      // to_address. The subquery filter keeps it to actual contract rows.
      const affected = new Set<string>();
      for (const c of rows.contracts) affected.add(c.address);
      for (const t of rows.txs) if (t.to_address) affected.add(t.to_address);
      if (affected.size > 0) {
        await client.query(
          "UPDATE evm_contracts c SET transaction_count =" +
            " (SELECT count(*) FROM evm_transactions x WHERE x.to_address = c.address)" +
            " WHERE c.address = ANY($1)",
          [[...affected]],
        );
      }

      await client.query("COMMIT");
    } catch (err) {
      await client.query("ROLLBACK");
      throw err;
    } finally {
      client.release();
    }
  }

  async function upsertBalances(rows: readonly BalanceRow[]): Promise<void> {
    if (rows.length === 0) return;
    const client = await pool.connect();
    try {
      // Single transaction across all chunks: the seed scan can produce
      // thousands of rows; a mid-way failure must not leave a partial snapshot.
      await client.query("BEGIN");
      await insertRows(
        client,
        "balance_accounts",
        BALANCE_COLUMNS,
        rows.map((b) => [b.address, b.balance, b.locked, b.reserved, b.nonce]),
        " ON CONFLICT (address) DO UPDATE SET" +
          " balance = EXCLUDED.balance," +
          " locked = EXCLUDED.locked," +
          " reserved = EXCLUDED.reserved," +
          " nonce = EXCLUDED.nonce",
      );
      await client.query("COMMIT");
    } catch (err) {
      await client.query("ROLLBACK");
      throw err;
    } finally {
      client.release();
    }
  }

  async function close(): Promise<void> {
    await pool.end();
  }

  return { init, writeBatch, upsertBalances, close };
}
