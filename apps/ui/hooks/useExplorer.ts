"use client";

import { useQuery } from "@tanstack/react-query";
import { SQUID_URL } from "~/config/indexer";

// Indexed EVM block/tx data from the Subsquid GraphQL (OpenReader). Powers the
// block explorer's full-history views (block list/detail, tx detail, and the
// per-address tx history that the RPC alone cannot serve).

export interface ExplorerBlock {
  id: string;
  height: number;
  hash: string;
  parentHash: string;
  timestamp: string;
  author: string | null;
  gasUsed: string;
  gasLimit: string;
  size: string;
  txCount: number;
  baseFeePerGas: string | null;
}

export interface ExplorerTx {
  id: string; // tx hash
  block: number;
  timestamp: string;
  txIndex: number;
  from: string;
  to: string | null;
  value: string;
  input: string;
  nonce: number;
  gasUsed: string;
  gasPrice: string;
  effectiveGasPrice: string;
  cumulativeGasUsed: string;
  maxFeePerGas: string | null;
  maxPriorityFeePerGas: string | null;
  txType: number | null;
  success: boolean;
  contractCreated: string | null;
}

export interface ExplorerBalance {
  free: string;
  reserved: string;
  frozen: string;
  total: string;
  nonce: number;
}

async function gql<T>(query: string, variables?: Record<string, unknown>): Promise<T> {
  const res = await fetch(`${SQUID_URL}/graphql`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });
  const json = await res.json();
  if (json.errors?.length) {
    throw new Error(json.errors[0]?.message ?? "GraphQL error");
  }
  return json.data as T;
}

const BLOCK_FIELDS = `id height hash parentHash timestamp author gasUsed gasLimit size txCount baseFeePerGas`;
const TX_FIELDS = `id block timestamp txIndex from to value input nonce gasUsed gasPrice effectiveGasPrice cumulativeGasUsed maxFeePerGas maxPriorityFeePerGas txType success contractCreated`;
const TX_SUMMARY_FIELDS = `id block timestamp txIndex from to value success`;

export function useExplorerBlocks(limit = 25, offset = 0) {
  return useQuery({
    queryKey: ["explorer", "blocks", limit, offset],
    queryFn: () =>
      gql<{ blocks: ExplorerBlock[]; blocksConnection: { totalCount: number } }>(
        `query Blocks($limit: Int!, $offset: Int!) {
          blocks(orderBy: [height_DESC], limit: $limit, offset: $offset) { ${BLOCK_FIELDS} }
          blocksConnection(orderBy: height_ASC) { totalCount }
        }`,
        { limit, offset },
      ),
    refetchInterval: 12000,
  });
}

export function useExplorerBlock(height: number | null) {
  return useQuery({
    queryKey: ["explorer", "block", height],
    enabled: height != null && Number.isFinite(height),
    queryFn: () =>
      gql<{ blockById: ExplorerBlock | null }>(
        `query Block($id: String!) { blockById(id: $id) { ${BLOCK_FIELDS} } }`,
        { id: String(height) },
      ).then((d) => d.blockById),
  });
}

export function useExplorerBlockByHash(hash: string | null) {
  return useQuery({
    queryKey: ["explorer", "blockByHash", hash],
    enabled: !!hash,
    queryFn: () =>
      gql<{ blocks: ExplorerBlock[] }>(
        `query BlockByHash($hash: String!) {
          blocks(where: { hash_eq: $hash }, limit: 1) { ${BLOCK_FIELDS} }
        }`,
        { hash },
      ).then((d) => d.blocks[0] ?? null),
  });
}

export function useExplorerBlockTxs(height: number | null) {
  return useQuery({
    queryKey: ["explorer", "blockTxs", height],
    enabled: height != null && Number.isFinite(height),
    queryFn: () =>
      gql<{ evmTransactions: ExplorerTx[] }>(
        `query BlockTxs($block: Int!) {
          evmTransactions(where: { block_eq: $block }, orderBy: [txIndex_ASC], limit: 500) { ${TX_FIELDS} }
        }`,
        { block: height },
      ).then((d) => d.evmTransactions),
  });
}

export function useExplorerTx(hash: string | null) {
  return useQuery({
    queryKey: ["explorer", "tx", hash],
    enabled: !!hash,
    queryFn: () =>
      gql<{ evmTransactionById: ExplorerTx | null }>(
        `query Tx($id: String!) { evmTransactionById(id: $id) { ${TX_FIELDS} } }`,
        { id: hash },
      ).then((d) => d.evmTransactionById),
  });
}

export function useExplorerTxs(limit = 25, offset = 0) {
  return useQuery({
    queryKey: ["explorer", "txs", limit, offset],
    queryFn: () =>
      gql<{
        evmTransactions: ExplorerTx[];
        evmTransactionsConnection: { totalCount: number };
      }>(
        `query Txs($limit: Int!, $offset: Int!) {
          evmTransactions(orderBy: [block_DESC, txIndex_DESC], limit: $limit, offset: $offset) { ${TX_FIELDS} }
          evmTransactionsConnection(orderBy: block_ASC) { totalCount }
        }`,
        { limit, offset },
      ),
    refetchInterval: 12000,
  });
}

export function useAddressTxs(address: string | null, limit = 25, offset = 0) {
  const addr = address?.toLowerCase() ?? null;
  return useQuery({
    queryKey: ["explorer", "addressTxs", addr, limit, offset],
    enabled: !!addr,
    queryFn: () =>
      gql<{
        evmTransactions: ExplorerTx[];
        evmTransactionsConnection: { totalCount: number };
      }>(
        `query AddressTxs($addr: String!, $limit: Int!, $offset: Int!) {
          evmTransactions(
            where: { OR: [{ from_eq: $addr }, { to_eq: $addr }] }
            orderBy: [block_DESC, txIndex_DESC]
            limit: $limit
            offset: $offset
          ) { ${TX_SUMMARY_FIELDS} }
          evmTransactionsConnection(orderBy: block_ASC, where: { OR: [{ from_eq: $addr }, { to_eq: $addr }] }) { totalCount }
        }`,
        { addr, limit, offset },
      ),
  });
}

export function useAddressBalance(address: string | null) {
  const addr = address?.toLowerCase() ?? null;
  return useQuery({
    queryKey: ["explorer", "balance", addr],
    enabled: !!addr,
    queryFn: () =>
      gql<{ holderById: ExplorerBalance | null }>(
        `query Balance($id: String!) {
          holderById(id: $id) { free reserved frozen total nonce }
        }`,
        { id: addr },
      ).then((d) => d.holderById),
  });
}
