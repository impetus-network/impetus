"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { type Address } from "viem";
import { useAccount } from "wagmi";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { Button } from "@artemis/coss-ui/ui/button";
import { ClayButton, ClayCard } from "@artemis/coss-ui/clay";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";

export function ValidatorManage(): ReactElement {
  const { address } = useAccount();
  const staking = useScaffoldWriteContract("Staking");
  const session = useScaffoldWriteContract("Session");
  const [commission, setCommission] = useState("");
  const [error, setError] = useState("");

  const prefs = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "validators",
    args: address ? [address as Address] : undefined,
    enabled: !!address,
  });
  const prefsData = prefs.data as readonly [number, boolean] | undefined;
  const isMining = staking.isMining || session.isMining;

  async function handleCommission(e: FormEvent): Promise<void> {
    e.preventDefault();
    setError("");
    const n = Number(commission);
    if (!Number.isInteger(n) || n < 0 || n > 100) {
      setError("Commission must be a whole number 0–100");
      return;
    }
    try {
      await staking.writeAsync("validate", [{ commissionPercent: n, blocked: false }]);
      setCommission("");
      prefs.refetch();
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <ClayCard>
      <div className="flex flex-col gap-5">
        <div className="flex flex-wrap gap-6">
          <div>
            <p className="text-[11px] font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">
              Current commission
            </p>
            <p className="font-mono text-2xl font-bold">
              {prefsData ? `${prefsData[0]}%` : "—"}
            </p>
          </div>
          <div>
            <p className="text-[11px] font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">Status</p>
            <p className="font-mono text-2xl font-bold">
              {prefsData ? (prefsData[1] ? "blocked" : "open") : "—"}
            </p>
          </div>
        </div>

        <form onSubmit={handleCommission} className="flex items-end gap-3 border-t border-[#ece5d6] pt-4">
          <Field invalid={!!error} className="flex-1">
            <FieldLabel>Update commission (%)</FieldLabel>
            <Input
              type="text"
              inputMode="numeric"
              placeholder="5"
              value={commission}
              onChange={(e) => {
                setCommission(e.target.value);
                setError("");
              }}
              className="max-w-[160px] font-mono"
            />
            {error && <FieldError>{error}</FieldError>}
          </Field>
          <ClayButton type="submit" disabled={isMining}>
            Update
          </ClayButton>
        </form>

        <div className="flex flex-wrap gap-2 border-t border-[#ece5d6] pt-4">
          <Button
            type="button"
            variant="outline"
            disabled={isMining}
            onClick={() => staking.writeAsync("chill", []).then(() => prefs.refetch()).catch(() => undefined)}
          >
            Stop validating (chill)
          </Button>
          <Button
            type="button"
            variant="destructive"
            disabled={isMining}
            onClick={() => session.writeAsync("purgeKeys", []).catch(() => undefined)}
          >
            Purge session keys
          </Button>
        </div>
        <p className="text-[12px] text-[#6a6a6a]">
          Chilling removes you from the next election but keeps your bond. Purge keys only after you
          have stopped validating and your node is offline.
        </p>
      </div>
    </ClayCard>
  );
}
