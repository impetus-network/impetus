// Maps canonical eth-RPC responses into the exact row shapes of the four
// Subscan-style explorer tables. All numeric/bigint columns are carried as
// decimal STRINGS (never JS numbers) so 256-bit wei values never lose
// precision; pg binds them straight into numeric/int8 columns.

import {
  EthRpc,
  mapLimit,
  type EvmRpcBlock,
  type EvmRpcReceipt,
  type EvmRpcTx,
} from "./evm-rpc";

// Stride between blocks for the synthetic, deterministic transaction_id
// (block_num * STRIDE + transaction_index). Reorg-stable (same tx -> same id),
// monotonic with chain order, and < 2^53 for any realistic height (the
// explorer's safeNum throws above Number.MAX_SAFE_INTEGER). 1e5 >> max txs/block.
export const TX_ID_STRIDE = 100_000n;

export interface EvmBlockRow {
  block_num: string;
  block_hash: string | null;
  parent_hash: string | null;
  sha3_uncles: string | null;
  author: string | null;
  miner: string | null;
  state_root: string | null;
  transactions_root: string | null;
  receipts_root: string | null;
  gas_used: string | null;
  gas_limit: string | null;
  extra_data: string | null;
  logs_bloom: string | null;
  timestamp: string | null;
  block_size: string | null;
  transaction_count: number;
  base_fee_per_gas: string | null;
}

export interface EvmTxRow {
  hash: string;
  block_num: string;
  block_timestamp: string;
  from_address: string;
  to_address: string | null;
  input_data: string;
  nonce: string;
  gas_limit: string;
  gas_price: string;
  gas_used: string;
  contract: string | null;
  success: boolean;
  r: string | null;
  s: string | null;
  v: string | null;
  value: string;
  effective_gas_price: string;
  max_priority_fee_per_gas: string;
  max_fee_per_gas: string;
  cumulative_gas_used: string;
  txn_type: string | null;
  transaction_index: string;
  transaction_id: string;
}

export interface EvmContractRow {
  address: string;
  deployer: string;
  block_num: string;
  tx_hash: string;
  deploy_at: string;
  creation_bytecode: string;
}

export interface EvmBatchRows {
  blocks: EvmBlockRow[];
  txs: EvmTxRow[];
  contracts: EvmContractRow[];
}

/** hex quantity -> decimal string (defaults to "0" for null/undefined). */
function hb(h: string | null | undefined): string {
  return h == null ? "0" : BigInt(h).toString();
}

/** hex quantity -> decimal string, preserving null. */
function hbNull(h: string | null | undefined): string | null {
  return h == null ? null : BigInt(h).toString();
}

const lc = (s: string): string => s.toLowerCase();

function mapBlock(b: EvmRpcBlock): EvmBlockRow {
  return {
    block_num: hb(b.number),
    block_hash: b.hash ?? null,
    parent_hash: b.parentHash ?? null,
    sha3_uncles: b.sha3Uncles ?? null,
    author: b.miner ? lc(b.miner) : null,
    miner: b.miner ? lc(b.miner) : null,
    state_root: b.stateRoot ?? null,
    transactions_root: b.transactionsRoot ?? null,
    receipts_root: b.receiptsRoot ?? null,
    gas_used: hb(b.gasUsed),
    gas_limit: hb(b.gasLimit),
    extra_data: b.extraData ?? null,
    logs_bloom: b.logsBloom ?? null,
    timestamp: hb(b.timestamp),
    block_size: hbNull(b.size),
    transaction_count: b.transactions.length,
    base_fee_per_gas: hbNull(b.baseFeePerGas),
  };
}

function mapTx(
  height: number,
  b: EvmRpcBlock,
  tx: EvmRpcTx,
  receipt: EvmRpcReceipt,
): EvmTxRow {
  const txIndex = BigInt(tx.transactionIndex);
  const transactionId = (BigInt(height) * TX_ID_STRIDE + txIndex).toString();
  // effective_gas_price drives the explorer's fee (gas_used * effective_gas_price);
  // fall back to the legacy gasPrice if a receipt omits it.
  const effective =
    receipt.effectiveGasPrice != null
      ? hb(receipt.effectiveGasPrice)
      : hb(tx.gasPrice);
  return {
    hash: tx.hash,
    block_num: String(height),
    block_timestamp: hb(b.timestamp),
    from_address: lc(tx.from),
    to_address: tx.to ? lc(tx.to) : null,
    input_data: tx.input,
    nonce: hb(tx.nonce),
    gas_limit: hb(tx.gas),
    gas_price: hb(tx.gasPrice),
    gas_used: hb(receipt?.gasUsed),
    contract: receipt.contractAddress ? lc(receipt.contractAddress) : null,
    // A mined tx is success unless its receipt explicitly reports status 0x0.
    success: receipt.status !== "0x0",
    r: tx.r ?? null,
    s: tx.s ?? null,
    v: hbNull(tx.v),
    value: hb(tx.value),
    effective_gas_price: effective,
    max_priority_fee_per_gas:
      tx.maxPriorityFeePerGas != null ? hb(tx.maxPriorityFeePerGas) : "-1",
    max_fee_per_gas: hb(tx.maxFeePerGas),
    cumulative_gas_used: hb(receipt.cumulativeGasUsed),
    txn_type: hbNull(tx.type),
    transaction_index: txIndex.toString(),
    transaction_id: transactionId,
  };
}

function mapContract(
  height: number,
  b: EvmRpcBlock,
  tx: EvmRpcTx,
  receipt: EvmRpcReceipt,
): EvmContractRow {
  return {
    address: lc(receipt.contractAddress as string),
    deployer: lc(tx.from),
    block_num: String(height),
    tx_hash: tx.hash,
    deploy_at: hb(b.timestamp),
    creation_bytecode: tx.input,
  };
}

/**
 * Fetch and map the EVM data for every block height in the batch.
 * One eth_getBlockByNumber per block (needed for evm_blocks even when empty);
 * receipts fetched only for blocks that actually carry transactions.
 *
 * Squid delivers each block exactly once and this writer is outside its
 * hot-block rollback, so a null block or a missing receipt is NEVER skipped --
 * we THROW, which fails the batch and makes Squid retry it. Skipping would leave
 * a permanent hole in evm_blocks/evm_transactions or silently record a
 * receipt-less tx as success=true / gas_used=0.
 */
export async function buildEvmRows(
  eth: EthRpc,
  heights: readonly number[],
  concurrency = 10,
): Promise<EvmBatchRows> {
  const per = await mapLimit(heights, concurrency, async (height) => {
    const b = await eth.getBlock(height);
    if (!b) {
      throw new Error(`eth_getBlockByNumber returned null for height ${height}`);
    }
    const blockRow = mapBlock(b);
    const receipts = await eth.getBlockReceipts(
      height,
      b.transactions.map((t) => t.hash),
    );
    const byHash = new Map<string, EvmRpcReceipt>(
      receipts.map((r) => [r.transactionHash.toLowerCase(), r]),
    );
    const txRows: EvmTxRow[] = [];
    const contractRows: EvmContractRow[] = [];
    for (const tx of b.transactions) {
      const receipt = byHash.get(tx.hash.toLowerCase());
      if (!receipt) {
        throw new Error(
          `missing receipt for tx ${tx.hash} in block ${height}`,
        );
      }
      txRows.push(mapTx(height, b, tx, receipt));
      if (receipt.contractAddress) {
        contractRows.push(mapContract(height, b, tx, receipt));
      }
    }
    return { blockRow, txRows, contractRows };
  });

  const blocks: EvmBlockRow[] = [];
  const txs: EvmTxRow[] = [];
  const contracts: EvmContractRow[] = [];
  for (const p of per) {
    blocks.push(p.blockRow);
    txs.push(...p.txRows);
    contracts.push(...p.contractRows);
  }
  return { blocks, txs, contracts };
}
