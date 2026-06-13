import { createConfig } from "ponder";
import { http } from "viem";
import { GaslessRegistryAbi } from "./abis/GaslessRegistry";

// Impetus mainnet. PONDER_RPC_URL_388266 must be a node RPC the indexer can
// reach server-side (e.g. the archive service's internal URL) — NOT the browser
// rpc-proxy, which 403s non-impetus.network Origins and is pruned. Backfilling
// from block 0 needs an archive/full node that can serve historical bodies.
//
// NPoS (staking, nomination-pools) is NOT indexed here: the precompiles do not
// emit EVM logs, and pallet-staking / pallet-nomination-pools already emit rich
// Substrate events (Bonded, ValidatorPrefsSet, Chilled, PayoutStarted, Created,
// PaidOut, ...). Those are indexed by the separate Subsquid substrate indexer —
// see docs/substrate-indexer-plan.md. Ponder stays EVM-log-only (gasless).
const CHAIN_ID = 388266;

export default createConfig({
  networks: {
    impetus: {
      chainId: CHAIN_ID,
      transport: http(process.env.PONDER_RPC_URL_388266),
      disableCache: true,
      pollingInterval: 2_000,
    },
  },
  contracts: {
    GaslessRegistry: {
      network: "impetus",
      abi: GaslessRegistryAbi,
      address: "0x0000000000000000000000000000000000000800",
      startBlock: 0,
    },
  },
});
