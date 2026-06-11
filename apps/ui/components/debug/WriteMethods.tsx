"use client";

import { useState } from "react";
import { useWriteContract } from "wagmi";
import type { Abi, AbiFunction } from "viem";
import { useTransactor } from "~/hooks/useTransactor";

interface WriteMethodsProps {
  address: `0x${string}`;
  abi: readonly unknown[];
}

function WriteMethod({ address, abi, fn }: { address: `0x${string}`; abi: readonly unknown[]; fn: AbiFunction }) {
  const [args, setArgs] = useState<string[]>(fn.inputs.map(() => ""));
  const [sending, setSending] = useState(false);
  const { writeContractAsync } = useWriteContract();
  const { transact } = useTransactor();

  async function handleSubmit() {
    setSending(true);
    try {
      await transact(() =>
        writeContractAsync({
          address,
          abi: abi as Abi,
          functionName: fn.name,
          args: args.length > 0 ? args : undefined,
        }),
      );
    } catch {
      // Error shown via toast
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="rounded-lg border border-border p-4">
      <h4 className="font-mono text-sm font-medium">{fn.name}</h4>
      {fn.inputs.length > 0 && (
        <div className="mt-2 flex flex-col gap-2">
          {fn.inputs.map((input, i) => (
            <input
              key={i}
              type="text"
              placeholder={`${input.name || `arg${i}`} (${input.type})`}
              value={args[i]}
              onChange={(e) => {
                const next = [...args];
                next[i] = e.target.value;
                setArgs(next);
              }}
              className="rounded border border-border bg-background px-3 py-1.5 font-mono text-xs"
            />
          ))}
        </div>
      )}
      <button
        onClick={handleSubmit}
        disabled={sending}
        className="mt-2 rounded bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:opacity-90 disabled:opacity-50"
      >
        {sending ? "Sending..." : "Write"}
      </button>
    </div>
  );
}

export function WriteMethods({ address, abi }: WriteMethodsProps) {
  const writeFns = (abi as AbiFunction[]).filter(
    (item) => item.type === "function" && item.stateMutability === "nonpayable",
  );

  if (writeFns.length === 0) return <p className="text-sm text-muted-foreground">No write methods.</p>;

  return (
    <div className="flex flex-col gap-3">
      {writeFns.map((fn) => (
        <WriteMethod key={fn.name} address={address} abi={abi} fn={fn} />
      ))}
    </div>
  );
}
