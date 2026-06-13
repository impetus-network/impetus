"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { type Address, parseEther } from "viem";
import { useAccount } from "wagmi";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayButton, ClayCard } from "@artemis/coss-ui/clay";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";

export function CreatePoolForm(): ReactElement {
  const { address } = useAccount();
  const { writeAsync, isMining } = useScaffoldWriteContract("NominationPools");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState("");

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    setError("");
    if (!address) {
      setError("Connect your wallet");
      return;
    }
    if (!/^\d+(?:\.\d+)?$/.test(amount) || Number(amount) <= 0) {
      setError("Enter an amount greater than 0");
      return;
    }
    try {
      // You become root, nominator and bouncer of the new pool.
      const self = address as Address;
      await writeAsync("create", [parseEther(amount), self, self, self]);
      setAmount("");
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <ClayCard>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Field invalid={!!error}>
          <FieldLabel>Initial bond (IPT)</FieldLabel>
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
          Creates a new pool with you as root, nominator and bouncer. After creating, set its
          validators with the pool nominate call and let members join.
        </p>
        <ClayButton type="submit" disabled={isMining}>
          {isMining ? "Submitting..." : "Create pool"}
        </ClayButton>
      </form>
    </ClayCard>
  );
}
