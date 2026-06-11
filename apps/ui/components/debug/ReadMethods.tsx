"use client";

import { useState } from "react";
import { useReadContract } from "wagmi";
import type { Abi, AbiFunction } from "viem";

interface ReadMethodsProps {
  address: `0x${string}`;
  abi: readonly unknown[];
}

function ReadMethod({ address, abi, fn }: { address: `0x${string}`; abi: readonly unknown[]; fn: AbiFunction }) {
  const [args, setArgs] = useState<string[]>(fn.inputs.map(() => ""));
  const [enabled, setEnabled] = useState(false);

  const { data, isLoading, error } = useReadContract({
    address,
    abi: abi as Abi,
    functionName: fn.name,
    args: args.length > 0 ? args : undefined,
    query: { enabled },
  });

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
        onClick={() => setEnabled(true)}
        className="mt-2 rounded bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:opacity-90"
      >
        Read
      </button>
      {isLoading && <p className="mt-2 text-xs text-muted-foreground">Loading...</p>}
      {error && <p className="mt-2 text-xs text-destructive">{error.message.slice(0, 100)}</p>}
      {data !== undefined && (
        <pre className="mt-2 overflow-x-auto rounded bg-muted p-2 text-xs">
          {JSON.stringify(data, (_, v) => (typeof v === "bigint" ? v.toString() : v), 2)}
        </pre>
      )}
    </div>
  );
}

export function ReadMethods({ address, abi }: ReadMethodsProps) {
  const readFns = (abi as AbiFunction[]).filter(
    (item) => item.type === "function" && (item.stateMutability === "view" || item.stateMutability === "pure"),
  );

  if (readFns.length === 0) return <p className="text-sm text-muted-foreground">No read methods.</p>;

  return (
    <div className="flex flex-col gap-3">
      {readFns.map((fn) => (
        <ReadMethod key={fn.name} address={address} abi={abi} fn={fn} />
      ))}
    </div>
  );
}
