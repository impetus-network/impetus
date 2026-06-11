"use client";

import { type Hash, formatEther, formatGwei } from "viem";
import { useWalletClient, usePublicClient } from "wagmi";
import { toastManager } from "@artemis/coss-ui/ui/toast";
import { useTxConfirm, type TxConfirmDetails } from "~/components/providers/TxConfirmProvider";

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    if (error.message.includes("User rejected")) return "Transaction rejected by user";
    if (error.message.includes("insufficient funds")) return "Insufficient funds";
    const short = error.message.split("\n")[0];
    return short.length > 120 ? `${short.slice(0, 120)}...` : short;
  }
  return "Unexpected error";
}

export interface SimpleTxParams {
  to: `0x${string}`;
  value?: bigint;
  data?: `0x${string}`;
}

export interface GasEstimate {
  gas: bigint;
  gasPrice: bigint;
  totalCost: bigint;
  formatted: {
    gas: string;
    gasPrice: string;
    totalCost: string;
  };
}

type TxInput = (() => Promise<Hash>) | SimpleTxParams;

interface TransactOptions {
  skipConfirm?: boolean;
  functionName?: string;
}

interface UseTransactorReturn {
  transact: (tx: TxInput, options?: TransactOptions) => Promise<Hash | undefined>;
  estimate: (tx: TxInput) => Promise<GasEstimate | undefined>;
}

export function useTransactor(): UseTransactorReturn {
  const { data: walletClient } = useWalletClient();
  const publicClient = usePublicClient();
  const confirmTx = useTxConfirm();

  async function estimateGas(tx: TxInput): Promise<GasEstimate | undefined> {
    if (!publicClient || !walletClient) return undefined;

    try {
      let gas: bigint;

      if (typeof tx === "function") {
        gas = 200_000n;
      } else {
        gas = await publicClient.estimateGas({
          account: walletClient.account,
          to: tx.to,
          value: tx.value,
          data: tx.data,
        });
      }

      const gasPrice = await publicClient.getGasPrice();
      const totalCost = gas * gasPrice;

      return {
        gas,
        gasPrice,
        totalCost,
        formatted: {
          gas: gas.toString(),
          gasPrice: `${formatGwei(gasPrice)} Gwei`,
          totalCost: `${formatEther(totalCost)} ART`,
        },
      };
    } catch {
      return undefined;
    }
  }

  const estimate = async (tx: TxInput): Promise<GasEstimate | undefined> => {
    if (!publicClient || !walletClient) {
      toastManager.add({ title: "Wallet not connected", type: "error" });
      return undefined;
    }

    const loadingId = toastManager.add({
      title: "Estimating gas...",
      type: "loading",
    });

    const result = await estimateGas(tx);
    toastManager.close(loadingId);

    if (result) {
      toastManager.add({
        title: "Gas estimate",
        description: `${result.formatted.gas} gas — ${result.formatted.totalCost}`,
        type: "info",
      });
    } else {
      toastManager.add({ title: "Estimation failed", type: "error" });
    }

    return result;
  };

  const transact = async (tx: TxInput, options?: TransactOptions): Promise<Hash | undefined> => {
    if (!walletClient) {
      toastManager.add({ title: "Wallet not connected", type: "error" });
      return undefined;
    }

    // Estimate gas for confirmation dialog
    const gasEst = await estimateGas(tx);

    // Show confirmation dialog unless skipped
    if (!options?.skipConfirm) {
      const isSimpleTx = typeof tx !== "function";
      const details: TxConfirmDetails = {
        type: isSimpleTx ? "transfer" : "contract",
        to: isSimpleTx ? (tx as SimpleTxParams).to : undefined,
        value: isSimpleTx ? (tx as SimpleTxParams).value : undefined,
        functionName: options?.functionName,
        gasEstimate: gasEst,
      };

      const confirmed = await confirmTx(details);
      if (!confirmed) return undefined;
    }

    let loadingId: string | undefined;
    let transactionHash: Hash | undefined;

    try {
      loadingId = toastManager.add({
        title: "Awaiting confirmation",
        description: "Please confirm in your wallet...",
        type: "loading",
      });

      if (typeof tx === "function") {
        transactionHash = await tx();
      } else {
        transactionHash = await walletClient.sendTransaction(tx);
      }

      toastManager.close(loadingId);

      loadingId = toastManager.add({
        title: "Transaction sent",
        description: `${transactionHash.slice(0, 10)}...${transactionHash.slice(-6)}`,
        type: "loading",
      });

      const receipt = await publicClient?.waitForTransactionReceipt({
        hash: transactionHash,
      });

      toastManager.close(loadingId);

      if (receipt?.status === "reverted") {
        toastManager.add({
          title: "Transaction reverted",
          description: transactionHash,
          type: "error",
        });
        throw new Error("Transaction reverted");
      }

      toastManager.add({
        title: "Transaction confirmed",
        description: `${transactionHash.slice(0, 10)}...${transactionHash.slice(-6)}`,
        type: "success",
      });

      return transactionHash;
    } catch (error: unknown) {
      if (loadingId) toastManager.close(loadingId);

      const message = getErrorMessage(error);
      toastManager.add({ title: "Transaction failed", description: message, type: "error" });

      throw error;
    }
  };

  return { transact, estimate };
}
