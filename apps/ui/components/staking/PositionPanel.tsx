"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { formatEther, parseEther, zeroAddress } from "viem";
import { useAccount, useBalance } from "wagmi";
import { REWARD_DESTINATION_STAKED } from "@artemis/shared";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { type StakingStatus } from "~/hooks/useStakingStatus";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { Button } from "@artemis/coss-ui/ui/button";
import { ClayButton } from "@artemis/coss-ui/clay";
import { cn } from "~/lib/utils";

type Mode = "bond" | "unbond";

function isValidDecimalAmount(value: string): boolean {
  return /^\d+(?:\.\d+)?$/.test(value);
}

function formatIpt(value: number): string {
  return value.toLocaleString("en-US", { maximumFractionDigits: 4 });
}

interface PositionPanelProps {
  status: StakingStatus;
}

export function PositionPanel({ status }: PositionPanelProps): ReactElement {
  const { address } = useAccount();
  const { data: balance } = useBalance({ address });
  const { writeAsync, isMining } = useScaffoldWriteContract("Staking");
  const [mode, setMode] = useState<Mode>("bond");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState("");

  const balanceIpt = balance ? Number(formatEther(balance.value)) : 0;
  const unlockingIpt = Number(formatEther(status.totalWei - status.activeWei));
  const numeric = isValidDecimalAmount(amount) ? Number(amount) : 0;

  function selectMode(next: Mode): void {
    setMode(next);
    setAmount("");
    setError("");
  }

  function fillMax(): void {
    setAmount(mode === "bond" ? String(balanceIpt) : String(status.active));
    setError("");
  }

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    setError("");
    if (!isValidDecimalAmount(amount) || numeric <= 0) {
      setError("Enter an amount greater than 0");
      return;
    }
    const cap = mode === "bond" ? balanceIpt : status.active;
    if (numeric > cap) {
      setError(mode === "bond" ? "Insufficient balance" : "Exceeds active stake");
      return;
    }

    const valueWei = parseEther(amount);
    try {
      if (mode === "unbond") {
        await writeAsync("unbond", [valueWei]);
      } else if (status.isBonded) {
        await writeAsync("bondExtra", [valueWei]);
      } else {
        await writeAsync("bond", [
          valueWei,
          { kind: REWARD_DESTINATION_STAKED, account: zeroAddress },
        ]);
      }
      setAmount("");
      status.refetch();
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <div className="overflow-hidden rounded-[1.5rem] border border-[#ece5d6] bg-white">
      <div className="flex items-center justify-between border-b border-[#ece5d6] px-5 py-3.5">
        <h3 className="text-xs font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">Position</h3>
        <span
          className={cn(
            "rounded-full px-2.5 py-1 text-[11px] font-bold",
            status.isBonded ? "bg-[#22c55e]/15 text-[#15803d]" : "bg-[#f5f0e0] text-[#6a6a6a]",
          )}
        >
          {status.isBonded ? "Bonded" : "Not bonded"}
        </span>
      </div>

      <form onSubmit={handleSubmit} className="flex flex-col gap-3 p-5">
        <div className="flex rounded-xl bg-[#f5f0e0] p-1">
          {(["bond", "unbond"] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => selectMode(m)}
              disabled={m === "unbond" && !status.isBonded}
              className={cn(
                "flex-1 rounded-lg py-2 text-xs font-bold capitalize transition-colors disabled:opacity-40",
                mode === m ? "bg-white text-[#0a0a0a] shadow-sm" : "text-[#6a6a6a]",
              )}
            >
              {m}
            </button>
          ))}
        </div>

        <Field invalid={!!error}>
          <FieldLabel>{mode === "bond" ? "Amount to bond" : "Amount to unbond"} (IPT)</FieldLabel>
          <div className="relative">
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
            <button
              type="button"
              onClick={fillMax}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-[11px] font-extrabold text-[#ff4d8b]"
            >
              MAX
            </button>
          </div>
          {error && <FieldError>{error}</FieldError>}
        </Field>

        <p className="font-mono text-[12px] text-[#6a6a6a]">
          Active {formatIpt(status.active)} · Total {formatIpt(status.total)} · Unlocking{" "}
          {formatIpt(unlockingIpt)}
        </p>

        <ClayButton type="submit" disabled={isMining}>
          {isMining
            ? "Submitting..."
            : mode === "unbond"
              ? "Unbond"
              : status.isBonded
                ? "Bond extra"
                : "Bond IPT"}
        </ClayButton>

        {(status.isNominating || unlockingIpt > 0) && (
          <div className="flex flex-wrap gap-2 border-t border-[#ece5d6] pt-3">
            {status.isNominating && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={isMining}
                onClick={() =>
                  writeAsync("chill", [])
                    .then(() => status.refetch())
                    .catch(() => undefined)
                }
              >
                Stop nominating
              </Button>
            )}
            {unlockingIpt > 0 && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={isMining}
                onClick={() =>
                  writeAsync("withdrawUnbonded", [0])
                    .then(() => status.refetch())
                    .catch(() => undefined)
                }
              >
                Withdraw unbonded
              </Button>
            )}
          </div>
        )}
      </form>
    </div>
  );
}
