"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { formatEther, parseEther } from "viem";
import { useAccount, useBalance } from "wagmi";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayButton, ClayCard } from "@artemis/coss-ui/clay";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";

function isValidDecimalAmount(value: string): boolean {
  return /^\d+(?:\.\d+)?$/.test(value);
}

export function JoinPoolForm({ onJoined }: { onJoined?: () => void }): ReactElement {
  const { address } = useAccount();
  const { data: balance } = useBalance({ address });
  const { writeAsync, isMining } = useScaffoldWriteContract("NominationPools");
  const [poolId, setPoolId] = useState("");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState("");

  const balanceIpt = balance ? Number(formatEther(balance.value)) : 0;

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    setError("");
    if (!/^\d+$/.test(poolId) || Number(poolId) < 1) {
      setError("Enter a valid pool id");
      return;
    }
    if (!isValidDecimalAmount(amount) || Number(amount) <= 0) {
      setError("Enter an amount greater than 0");
      return;
    }
    if (Number(amount) > balanceIpt) {
      setError("Insufficient balance");
      return;
    }
    try {
      await writeAsync("join", [parseEther(amount), Number(poolId)]);
      setAmount("");
      onJoined?.();
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <ClayCard>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Field invalid={!!error}>
          <FieldLabel>Pool ID</FieldLabel>
          <Input
            type="text"
            inputMode="numeric"
            placeholder="1"
            value={poolId}
            onChange={(e) => {
              setPoolId(e.target.value);
              setError("");
            }}
            className="max-w-[120px] font-mono"
          />
        </Field>
        <Field invalid={!!error}>
          <FieldLabel>Amount to delegate (IPT)</FieldLabel>
          <Input
            type="text"
            inputMode="decimal"
            placeholder="0.0"
            value={amount}
            onChange={(e) => {
              setAmount(e.target.value);
              setError("");
            }}
            className="font-mono"
          />
          {error && <FieldError>{error}</FieldError>}
        </Field>
        <p className="text-[13px] text-[#6a6a6a]">
          Balance {balanceIpt.toLocaleString("en-US", { maximumFractionDigits: 4 })} IPT · pools let
          you stake below the nominator minimum and the pool nominates for you.
        </p>
        <ClayButton type="submit" disabled={isMining}>
          {isMining ? "Submitting..." : "Join pool"}
        </ClayButton>
      </form>
    </ClayCard>
  );
}
