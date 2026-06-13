"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { type Address, formatEther } from "viem";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayButton, ClayCard } from "@artemis/coss-ui/clay";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { KNOWN_VALIDATORS } from "~/config/validators";

export function PayoutTool(): ReactElement {
  const { writeAsync, isMining } = useScaffoldWriteContract("Staking");
  const [stash, setStash] = useState<Address>(KNOWN_VALIDATORS[0].stash);
  const [era, setEra] = useState("");
  const [error, setError] = useState("");

  const eraNum = /^\d+$/.test(era) ? Number(era) : NaN;
  const reward = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "erasValidatorReward",
    args: Number.isInteger(eraNum) ? [eraNum] : undefined,
    enabled: Number.isInteger(eraNum),
  });
  const rewardIpt =
    reward.data !== undefined ? Number(formatEther(reward.data as bigint)) : null;

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    setError("");
    if (!Number.isInteger(eraNum) || eraNum < 0) {
      setError("Enter a valid era number");
      return;
    }
    try {
      await writeAsync("payoutStakers", [stash, eraNum]);
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <ClayCard>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <p className="text-[13px] text-[#6a6a6a]">
          Payouts are permissionless — anyone can trigger a validator&apos;s era reward, which is
          split to its nominators by stake. Each era must be paid out within the history depth.
        </p>

        <Field>
          <FieldLabel>Validator</FieldLabel>
          <select
            value={stash}
            onChange={(e) => setStash(e.target.value as Address)}
            className="rounded-lg border border-[#ece5d6] bg-[#fffaf0] px-3 py-2 font-mono text-sm outline-none focus:border-[#0a0a0a]"
          >
            {KNOWN_VALIDATORS.map((v) => (
              <option key={v.stash} value={v.stash}>
                {v.name} · {v.stash.slice(0, 8)}…{v.stash.slice(-4)}
              </option>
            ))}
          </select>
        </Field>

        <Field invalid={!!error}>
          <FieldLabel>Era</FieldLabel>
          <Input
            type="text"
            inputMode="numeric"
            placeholder="e.g. 341"
            value={era}
            onChange={(e) => {
              setEra(e.target.value);
              setError("");
            }}
            className="max-w-[160px] font-mono"
          />
          {error && <FieldError>{error}</FieldError>}
        </Field>

        {Number.isInteger(eraNum) && (
          <p className="font-mono text-[13px] text-[#6a6a6a]">
            Era {eraNum} validator reward:{" "}
            {rewardIpt === null
              ? "—"
              : `${rewardIpt.toLocaleString("en-US", { maximumFractionDigits: 4 })} IPT`}
          </p>
        )}

        <ClayButton type="submit" disabled={isMining} className="w-fit">
          {isMining ? "Submitting..." : "Trigger payout"}
        </ClayButton>
      </form>
    </ClayCard>
  );
}
