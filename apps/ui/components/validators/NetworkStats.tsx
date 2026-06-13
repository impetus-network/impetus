"use client";

import { type ReactElement } from "react";
import { formatEther } from "viem";
import { ClayFeatureCard } from "@artemis/coss-ui/clay";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";

function num(value: unknown): string {
  return typeof value === "number" ? value.toLocaleString("en-US") : "...";
}

export function NetworkStats(): ReactElement {
  const activeEra = useScaffoldReadContract({ contractName: "Staking", functionName: "activeEra" });
  const counter = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "counterForValidators",
  });
  const target = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "validatorCount",
  });
  const minBond = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "minValidatorBond",
  });

  const eraIndex = (activeEra.data as readonly [number, bigint] | undefined)?.[0];
  const minBondIpt = minBond.data
    ? Number(formatEther(minBond.data as bigint)).toLocaleString("en-US")
    : "...";

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
      <ClayFeatureCard label="Active era" value={num(eraIndex)} tone="teal">
        ~1 hour per era
      </ClayFeatureCard>
      <ClayFeatureCard label="Registered validators" value={num(counter.data)} tone="lavender">
        intent to validate
      </ClayFeatureCard>
      <ClayFeatureCard label="Active slots" value={num(target.data)} tone="ochre">
        elected each era
      </ClayFeatureCard>
      <ClayFeatureCard label="Min validator bond" value={minBondIpt} tone="cream">
        IPT
      </ClayFeatureCard>
    </div>
  );
}
