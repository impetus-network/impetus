import {
  GaslessRegistryAbi,
  GASLESS_REGISTRY_ADDRESS,
  NominationPoolsAbi,
  NOMINATION_POOLS_PRECOMPILE_ADDRESS,
  SessionAbi,
  SESSION_PRECOMPILE_ADDRESS,
  StakingAbi,
  STAKING_PRECOMPILE_ADDRESS,
} from "@artemis/shared";

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
  {
    name: "Staking",
    address: STAKING_PRECOMPILE_ADDRESS,
    abi: StakingAbi,
  },
  {
    name: "Session",
    address: SESSION_PRECOMPILE_ADDRESS,
    abi: SessionAbi,
  },
  {
    name: "NominationPools",
    address: NOMINATION_POOLS_PRECOMPILE_ADDRESS,
    abi: NominationPoolsAbi,
  },
];
