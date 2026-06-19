// Maps canonical eth-RPC responses into Squid Block / EvmTransaction entities,
// which the TypeORM store persists (and rolls back automatically on reorg via
// supportHotBlocks) and the GraphQL server (OpenReader) serves to the dApp.

import {
  EthRpc,
  mapLimit,
  type EvmRpcBlock,
  type EvmRpcReceipt,
  type EvmRpcTx,
} from "./evm-rpc";
import { Block, EvmTransaction } from "./model";

export interface EvmBatchData {
  blocks: Block[];
  txs: EvmTransaction[];
}

/** hex quantity -> bigint (0n for null/undefined). */
function bi(h: string | null | undefined): bigint {
  return h == null ? 0n : BigInt(h);
}

/** hex quantity -> bigint, preserving null. */
function biNull(h: string | null | undefined): bigint | null {
  return h == null ? null : BigInt(h);
}

/** hex quantity -> number (only for small values: index, nonce, type). */
function num(h: string | null | undefined): number {
  return h == null ? 0 : Number(BigInt(h));
}

const lc = (s: string): string => s.toLowerCase();

function mapBlock(b: EvmRpcBlock): Block {
  const height = num(b.number);
  return new Block({
    id: String(height),
    height,
    hash: b.hash,
    parentHash: b.parentHash,
    timestamp: new Date(num(b.timestamp) * 1000),
    author: b.miner ? lc(b.miner) : null,
    gasUsed: bi(b.gasUsed),
    gasLimit: bi(b.gasLimit),
    size: bi(b.size),
    txCount: b.transactions.length,
    baseFeePerGas: biNull(b.baseFeePerGas),
  });
}

function mapTx(
  height: number,
  b: EvmRpcBlock,
  tx: EvmRpcTx,
  receipt: EvmRpcReceipt,
): EvmTransaction {
  // effective_gas_price drives the explorer's fee (gasUsed * effectiveGasPrice);
  // fall back to the legacy gasPrice if a receipt omits it.
  const effective =
    receipt.effectiveGasPrice != null
      ? bi(receipt.effectiveGasPrice)
      : bi(tx.gasPrice);
  return new EvmTransaction({
    id: tx.hash,
    block: height,
    timestamp: new Date(num(b.timestamp) * 1000),
    txIndex: num(tx.transactionIndex),
    from: lc(tx.from),
    to: tx.to ? lc(tx.to) : null,
    value: bi(tx.value),
    input: tx.input,
    nonce: num(tx.nonce),
    gasUsed: bi(receipt.gasUsed),
    gasPrice: bi(tx.gasPrice),
    effectiveGasPrice: effective,
    cumulativeGasUsed: bi(receipt.cumulativeGasUsed),
    maxFeePerGas: biNull(tx.maxFeePerGas),
    maxPriorityFeePerGas: biNull(tx.maxPriorityFeePerGas),
    txType: tx.type != null ? num(tx.type) : null,
    // A mined tx is success unless its receipt explicitly reports status 0x0.
    success: receipt.status !== "0x0",
    contractCreated: receipt.contractAddress ? lc(receipt.contractAddress) : null,
  });
}

/**
 * Fetch and map the EVM data for every block height in the batch.
 * One eth_getBlockByNumber per block (needed even for empty blocks);
 * receipts fetched only for blocks that carry transactions.
 *
 * A null block or a missing receipt is NEVER skipped -- we THROW so Squid
 * retries the batch (otherwise an idle RPC blip would drop a block/tx forever).
 */
export async function buildEvmData(
  eth: EthRpc,
  heights: readonly number[],
  concurrency = 10,
): Promise<EvmBatchData> {
  const per = await mapLimit(heights, concurrency, async (height) => {
    const b = await eth.getBlock(height);
    if (!b) {
      throw new Error(`eth_getBlockByNumber returned null for height ${height}`);
    }
    const block = mapBlock(b);
    const receipts = await eth.getBlockReceipts(
      height,
      b.transactions.map((t) => t.hash),
    );
    const byHash = new Map<string, EvmRpcReceipt>(
      receipts.map((r) => [r.transactionHash.toLowerCase(), r]),
    );
    const txs: EvmTransaction[] = [];
    for (const tx of b.transactions) {
      const receipt = byHash.get(tx.hash.toLowerCase());
      if (!receipt) {
        throw new Error(`missing receipt for tx ${tx.hash} in block ${height}`);
      }
      txs.push(mapTx(height, b, tx, receipt));
    }
    return { block, txs };
  });

  const blocks: Block[] = [];
  const txs: EvmTransaction[] = [];
  for (const p of per) {
    blocks.push(p.block);
    txs.push(...p.txs);
  }
  return { blocks, txs };
}
