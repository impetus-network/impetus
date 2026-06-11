"use client";

import { useState } from "react";
import { type Abi } from "viem";
import { useAccount, useWriteContract } from "wagmi";
import { useTransactor } from "./useTransactor";
import { debugContracts, type ContractConfig } from "~/config/contracts";
import { toastManager } from "@artemis/coss-ui/ui/toast";

type ContractName = (typeof debugContracts)[number]["name"];

function getContract(name: ContractName): ContractConfig | undefined {
  return debugContracts.find((c) => c.name === name);
}

interface UseScaffoldWriteContractReturn {
  writeAsync: (
    functionName: string,
    args?: readonly unknown[],
    value?: bigint,
  ) => Promise<`0x${string}` | undefined>;
  isMining: boolean;
}

export function useScaffoldWriteContract(contractName: ContractName): UseScaffoldWriteContractReturn {
  const { chain } = useAccount();
  const { writeContractAsync } = useWriteContract();
  const { transact } = useTransactor();
  const [isMining, setIsMining] = useState(false);

  const writeAsync = async (
    functionName: string,
    args?: readonly unknown[],
    value?: bigint,
  ): Promise<`0x${string}` | undefined> => {
    const contract = getContract(contractName);
    if (!contract) {
      toastManager.add({
        title: "Contract not found",
        description: `"${contractName}" is not configured in contracts.ts`,
        type: "error",
      });
      return undefined;
    }

    if (!chain?.id) {
      toastManager.add({ title: "Please connect your wallet", type: "error" });
      return undefined;
    }

    setIsMining(true);
    try {
      const result = await transact(
        () =>
          writeContractAsync({
            address: contract.address,
            abi: contract.abi as Abi,
            functionName,
            args: args ?? [],
            value,
          }),
        { functionName: `${contractName}.${functionName}` },
      );
      return result;
    } finally {
      setIsMining(false);
    }
  };

  return { writeAsync, isMining };
}
