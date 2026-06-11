import { useQuery } from "@tanstack/react-query";
import { createPublicClient, http } from "viem";
import { mainnet } from "viem/chains";
import { normalize } from "viem/ens";
import { useDebounce } from "./useDebounce";

interface EnsResolveResult {
  resolvedAddress: `0x${string}` | null;
  isResolving: boolean;
  ensError: string | null;
  isEnsInput: boolean;
}

const ETH_RPC_URL =
  process.env.NEXT_PUBLIC_ETH_MAINNET_RPC_URL ||
  "https://eth.public-rpc.com";

const ethClient = createPublicClient({
  chain: mainnet,
  transport: http(ETH_RPC_URL),
});

function isEnsName(value: string): boolean {
  return value.length >= 5 && value.endsWith(".eth");
}

function toUserMessage(error: unknown): string {
  if (error instanceof Error) {
    const msg = error.message;
    if (msg.includes("No address found")) return msg;
    if (msg.includes("not a valid ENS")) return msg;
  }
  return "Could not resolve ENS name. Please try again later.";
}

async function resolveEns(name: string): Promise<`0x${string}`> {
  let normalized: string;
  try {
    normalized = normalize(name);
  } catch {
    throw new Error(`"${name}" is not a valid ENS name`);
  }

  const address = await ethClient.getEnsAddress({ name: normalized });

  if (!address) {
    throw new Error(`No address found for "${normalized}"`);
  }

  return address;
}

export function useEnsResolve(input: string): EnsResolveResult {
  const isEns = isEnsName(input.trim());
  const debouncedName = useDebounce(input.trim(), 500);
  const enabled = isEns && isEnsName(debouncedName);

  const { data, isLoading, error } = useQuery({
    queryKey: ["ens", debouncedName],
    queryFn: () => resolveEns(debouncedName),
    enabled,
    staleTime: 5 * 60 * 1000,
    retry: false,
  });

  const isStale = isEns && debouncedName !== input.trim();

  return {
    resolvedAddress: enabled ? (data ?? null) : null,
    isResolving: isStale || (enabled && isLoading),
    ensError: enabled && error ? toUserMessage(error) : null,
    isEnsInput: isEns,
  };
}
