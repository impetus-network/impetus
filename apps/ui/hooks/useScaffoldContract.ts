"use client";

import { type Abi, getContract, type GetContractReturnType, type Transport, type Chain, type Account } from "viem";
import { usePublicClient, useWalletClient } from "wagmi";
import { debugContracts } from "~/config/contracts";

type ContractName = (typeof debugContracts)[number]["name"];

interface UseScaffoldContractParams {
  contractName: ContractName;
}

export function useScaffoldContract({ contractName }: UseScaffoldContractParams) {
  const publicClient = usePublicClient();
  const { data: walletClient } = useWalletClient();

  const contractConfig = debugContracts.find((c) => c.name === contractName);

  if (!contractConfig || !publicClient) {
    return { data: undefined, isLoading: !contractConfig };
  }

  const contract = getContract({
    address: contractConfig.address,
    abi: contractConfig.abi as Abi,
    client: walletClient ?? publicClient,
  });

  return {
    data: contract as GetContractReturnType<Abi, { public: typeof publicClient; wallet: typeof walletClient }, `0x${string}`>,
    isLoading: false,
  };
}
