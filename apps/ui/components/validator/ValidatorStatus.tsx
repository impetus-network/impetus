"use client";

import { type ReactElement } from "react";
import { type Address, formatEther } from "viem";
import { useAccount } from "wagmi";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { useStakingStatus } from "~/hooks/useStakingStatus";

interface ChecklistItemProps {
  done: boolean;
  label: string;
  detail: string;
}

function ChecklistItem({ done, label, detail }: ChecklistItemProps): ReactElement {
  return (
    <li className="flex items-start gap-3 py-2.5">
      <span
        className={
          done
            ? "mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full bg-[#22c55e] text-[11px] font-bold text-white"
            : "mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full border border-[#d8cfba] text-[11px] text-[#b3a991]"
        }
      >
        {done ? "✓" : ""}
      </span>
      <div className="flex flex-col">
        <span className="text-sm font-semibold text-[#0a0a0a]">{label}</span>
        <span className="text-[13px] text-[#6a6a6a]">{detail}</span>
      </div>
    </li>
  );
}

export function ValidatorStatus(): ReactElement {
  const { address } = useAccount();
  const status = useStakingStatus();

  const minValidatorBond = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "minValidatorBond",
  });
  const nextKeys = useScaffoldReadContract({
    contractName: "Session",
    functionName: "nextKeys",
    args: address ? [address as Address] : undefined,
    enabled: !!address,
  });
  const prefs = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "validators",
    args: address ? [address as Address] : undefined,
    enabled: !!address,
  });

  const minBondIpt = minValidatorBond.data
    ? Number(formatEther(minValidatorBond.data as bigint))
    : 1000;
  const bondedEnough = status.active >= minBondIpt;
  const keysSet = typeof nextKeys.data === "string" && nextKeys.data.length > 2;
  const prefsData = prefs.data as readonly [number, boolean] | undefined;
  // `validators(addr)` has no dedicated "is validator" view and returns
  // (0,false) by default, so we only treat a confirmed, non-erroring read
  // alongside registered keys as an active-validator hint.
  const validating = keysSet && !prefs.isError && prefsData !== undefined;

  return (
    <div className="rounded-[1.5rem] border border-[#ece5d6] bg-white p-5">
      <h3 className="mb-1 text-xs font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">
        Your readiness
      </h3>
      <ul className="divide-y divide-[#f2ece0]">
        <ChecklistItem
          done={bondedEnough}
          label={`Bonded ≥ ${minBondIpt.toLocaleString("en-US")} IPT`}
          detail={`Active stake: ${status.active.toLocaleString("en-US", { maximumFractionDigits: 2 })} IPT`}
        />
        <ChecklistItem
          done={keysSet}
          label="Session keys registered"
          detail={keysSet ? "Keys are queued on-chain" : "Run author_rotateKeys, then setKeys below"}
        />
        <ChecklistItem
          done={validating}
          label="Validating"
          detail={
            validating && prefsData
              ? `Commission ${prefsData[0]}%${prefsData[1] ? " · blocked" : ""}`
              : "Call validate() once bonded and keys are set"
          }
        />
      </ul>
    </div>
  );
}
