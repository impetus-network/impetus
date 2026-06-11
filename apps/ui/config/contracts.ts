import { GaslessRegistryAbi, GASLESS_REGISTRY_ADDRESS } from "@artemis/shared";

export interface ContractConfig {
  name: string;
  address: `0x${string}`;
  abi: readonly unknown[];
}

export const debugContracts: ContractConfig[] = [
  {
    name: "GaslessRegistry",
    address: GASLESS_REGISTRY_ADDRESS,
    abi: GaslessRegistryAbi,
  },
];
