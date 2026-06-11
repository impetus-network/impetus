"use client";

import { type Abi } from "viem";
import { useReadContract } from "wagmi";
import { debugContracts, type ContractConfig } from "~/config/contracts";

type ContractName = (typeof debugContracts)[number]["name"];

function getContract(name: ContractName): ContractConfig | undefined {
  return debugContracts.find((c) => c.name === name);
}

interface UseScaffoldReadContractParams {
  contractName: ContractName;
  functionName: string;
  args?: readonly unknown[];
  enabled?: boolean;
}

export function useScaffoldReadContract({
  contractName,
  functionName,
  args,
  enabled = true,
}: UseScaffoldReadContractParams) {
  const contract = getContract(contractName);

  return useReadContract({
    address: contract?.address,
    abi: (contract?.abi ?? []) as Abi,
    functionName,
    args: args ?? [],
    query: { enabled: enabled && !!contract },
  });
}
