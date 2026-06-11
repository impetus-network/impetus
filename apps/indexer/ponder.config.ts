import { createConfig } from "ponder";
import { http } from "viem";
import { GaslessRegistryAbi } from "./abis/GaslessRegistry";

export default createConfig({
  networks: {
    artemis: {
      chainId: 322644,
      transport: http(process.env.PONDER_RPC_URL_322644),
      disableCache: true,
      pollingInterval: 2_000,
    },
  },
  contracts: {
    GaslessRegistry: {
      network: "artemis",
      abi: GaslessRegistryAbi,
      address: "0x0000000000000000000000000000000000000800",
      startBlock: 0,
    },
  },
});
