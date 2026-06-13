"use client";

import { type ReactElement, useState } from "react";
import { type Address, formatEther, parseEther } from "viem";
import { useAccount } from "wagmi";
import { Field, FieldLabel } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { Button } from "@artemis/coss-ui/ui/button";
import { ClayButton, ClayCard, ClayEmptyState } from "@artemis/coss-ui/clay";
import { BOND_EXTRA_REWARDS } from "@artemis/shared";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";

export function PoolMemberPanel(): ReactElement {
  const { address } = useAccount();
  const { writeAsync, isMining } = useScaffoldWriteContract("NominationPools");
  const [unbondPoints, setUnbondPoints] = useState("");

  const member = useScaffoldReadContract({
    contractName: "NominationPools",
    functionName: "poolMembers",
    args: address ? [address as Address] : undefined,
    enabled: !!address,
  });

  const data = member.data as
    | readonly [number, bigint, bigint, readonly unknown[]]
    | undefined;
  const poolId = data?.[0] ?? 0;
  const points = data?.[1] ?? 0n;
  const isMember = poolId > 0 && points > 0n;

  function run(fn: string, args: readonly unknown[]): void {
    writeAsync(fn, args)
      .then(() => member.refetch())
      .catch(() => undefined);
  }

  if (!isMember) {
    return (
      <ClayEmptyState
        title="Not in a pool"
        description="Join a pool below to start earning. Your membership and rewards will show here."
      />
    );
  }

  return (
    <ClayCard>
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap gap-6">
          <div>
            <p className="text-[11px] font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">Pool</p>
            <p className="font-mono text-2xl font-bold">#{poolId}</p>
          </div>
          <div>
            <p className="text-[11px] font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">
              Your points
            </p>
            <p className="font-mono text-2xl font-bold">
              {Number(formatEther(points)).toLocaleString("en-US", { maximumFractionDigits: 2 })}
            </p>
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          <ClayButton type="button" disabled={isMining} onClick={() => run("claimPayout", [])}>
            Claim rewards
          </ClayButton>
          <Button
            type="button"
            variant="outline"
            disabled={isMining}
            onClick={() => run("bondExtra", [{ kind: BOND_EXTRA_REWARDS, amount: 0n }])}
          >
            Compound rewards
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={isMining}
            onClick={() => run("withdrawUnbonded", [address as Address, 0])}
          >
            Withdraw unbonded
          </Button>
        </div>

        <div className="flex items-end gap-3 border-t border-[#ece5d6] pt-4">
          <Field className="flex-1">
            <FieldLabel>Unbond points</FieldLabel>
            <Input
              type="text"
              inputMode="decimal"
              placeholder="0.0"
              value={unbondPoints}
              onChange={(e) => setUnbondPoints(e.target.value)}
              className="font-mono"
            />
          </Field>
          <Button
            type="button"
            variant="outline"
            disabled={isMining || !/^\d+(?:\.\d+)?$/.test(unbondPoints)}
            onClick={() => run("unbond", [address as Address, parseEther(unbondPoints || "0")])}
          >
            Unbond
          </Button>
        </div>
        <p className="text-[12px] text-[#6a6a6a]">
          Unbonded funds are locked for the ~28-day bonding period, then withdrawable.
        </p>
      </div>
    </ClayCard>
  );
}
