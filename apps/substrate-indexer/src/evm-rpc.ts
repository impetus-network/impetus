// Minimal Ethereum JSON-RPC client over the SAME node the Squid processor
// ingests from. Impetus is a Frontier chain, so the archive node serves the
// `eth_*` namespace on the same endpoint as the substrate `state_*`/`chain_*`
// RPC (RPC_ENDPOINT). We source the explorer's EVM block/tx/receipt data here
// via canonical eth RPC instead of SCALE-decoding pallet_ethereum -- no typegen
// refresh, no metadata-drift risk, and exact Ethereum semantics.
//
// Plain `fetch` (global in Node >= 20); no viem/axios dependency added.

export interface EvmRpcTx {
  hash: string;
  from: string;
  to: string | null;
  value: string; // hex wei
  input: string;
  nonce: string; // hex
  gas: string; // hex (gas limit)
  gasPrice?: string; // hex
  maxFeePerGas?: string; // hex
  maxPriorityFeePerGas?: string; // hex
  type?: string; // hex
  transactionIndex: string; // hex
  r?: string;
  s?: string;
  v?: string; // hex
}

export interface EvmRpcBlock {
  number: string; // hex
  hash: string;
  parentHash: string;
  sha3Uncles?: string;
  miner: string;
  stateRoot?: string;
  transactionsRoot?: string;
  receiptsRoot?: string;
  gasUsed: string; // hex
  gasLimit: string; // hex
  extraData?: string;
  logsBloom?: string;
  timestamp: string; // hex seconds
  size?: string; // hex
  baseFeePerGas?: string; // hex
  transactions: EvmRpcTx[];
}

export interface EvmRpcReceipt {
  transactionHash: string;
  transactionIndex: string; // hex
  gasUsed: string; // hex
  cumulativeGasUsed: string; // hex
  effectiveGasPrice?: string; // hex
  contractAddress: string | null;
  status?: string; // hex 0x1 / 0x0
}

interface JsonRpcResponse<T> {
  jsonrpc: string;
  id: number;
  result?: T;
  error?: { code: number; message: string };
}

const METHOD_NOT_FOUND = -32601;

function toHexQuantity(n: number): string {
  return "0x" + n.toString(16);
}

export class EthRpc {
  private readonly url: string;
  private readonly maxRetries: number;
  private id = 0;
  // Cache whether eth_getBlockReceipts is supported (Frontier exposes it on
  // recent fc-rpc; older nodes fall back to per-tx eth_getTransactionReceipt).
  private blockReceiptsSupported: boolean | undefined;

  constructor(url: string, maxRetries = 5) {
    this.url = url;
    this.maxRetries = maxRetries;
  }

  private async rpc<T>(method: string, params: unknown[]): Promise<T> {
    let lastErr: unknown;
    for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
      try {
        const res = await fetch(this.url, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            jsonrpc: "2.0",
            id: ++this.id,
            method,
            params,
          }),
        });
        if (!res.ok) {
          throw new Error(`eth rpc ${method} HTTP ${res.status}`);
        }
        const body = (await res.json()) as JsonRpcResponse<T>;
        if (body.error) {
          const err = new Error(`eth rpc ${method}: ${body.error.message}`);
          (err as { code?: number }).code = body.error.code;
          throw err;
        }
        return body.result as T;
      } catch (err) {
        lastErr = err;
        // Don't retry a genuine "method not found" -- surface immediately.
        if ((err as { code?: number }).code === METHOD_NOT_FOUND) throw err;
        if (attempt < this.maxRetries) {
          await sleep(Math.min(200 * 2 ** attempt, 4000));
        }
      }
    }
    throw lastErr instanceof Error ? lastErr : new Error(String(lastErr));
  }

  /** eth_getBlockByNumber(height, fullTransactions=true). */
  async getBlock(height: number): Promise<EvmRpcBlock | null> {
    return this.rpc<EvmRpcBlock | null>("eth_getBlockByNumber", [
      toHexQuantity(height),
      true,
    ]);
  }

  /**
   * All receipts for a block. Uses eth_getBlockReceipts when available
   * (one round-trip), else falls back to N eth_getTransactionReceipt.
   */
  async getBlockReceipts(
    height: number,
    txHashes: string[],
  ): Promise<EvmRpcReceipt[]> {
    if (txHashes.length === 0) return [];

    if (this.blockReceiptsSupported !== false) {
      try {
        const receipts = await this.rpc<EvmRpcReceipt[] | null>(
          "eth_getBlockReceipts",
          [toHexQuantity(height)],
        );
        if (receipts != null) {
          this.blockReceiptsSupported = true;
          return receipts;
        }
        // Null result -> treat as unsupported and stop retrying the batch
        // method; fall through to per-tx receipts (which throw on a null).
        this.blockReceiptsSupported = false;
      } catch (err) {
        if ((err as { code?: number }).code === METHOD_NOT_FOUND) {
          this.blockReceiptsSupported = false;
        } else {
          throw err;
        }
      }
    }

    // Fallback: per-transaction receipts. A null receipt for a tx the node just
    // reported in the block is a transient condition -- THROW so the batch
    // retries rather than silently writing gas_used=0 / success=true.
    const out: EvmRpcReceipt[] = [];
    for (const hash of txHashes) {
      const r = await this.rpc<EvmRpcReceipt | null>(
        "eth_getTransactionReceipt",
        [hash],
      );
      if (!r) {
        throw new Error(`eth_getTransactionReceipt returned null for ${hash}`);
      }
      out.push(r);
    }
    return out;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Map over items with bounded concurrency (preserves input order). */
export async function mapLimit<T, R>(
  items: readonly T[],
  limit: number,
  fn: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  const results = new Array<R>(items.length);
  let next = 0;
  const workers = Array.from({ length: Math.min(limit, items.length) }, () =>
    (async () => {
      while (true) {
        const i = next++;
        if (i >= items.length) break;
        results[i] = await fn(items[i]!, i);
      }
    })(),
  );
  await Promise.all(workers);
  return results;
}
