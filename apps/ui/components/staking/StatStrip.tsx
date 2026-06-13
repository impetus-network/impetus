"use client";

import { type ReactElement } from "react";
import { formatEther } from "viem";
import { ClayFeatureCard } from "@artemis/coss-ui/clay";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { type StakingStatus } from "~/hooks/useStakingStatus";

function formatIpt(value: number): string {
  return value.toLocaleString("en-US", { maximumFractionDigits: value >= 100 ? 0 : 4 });
}

interface StatStripProps {
  status: StakingStatus;
}

export function StatStrip({ status }: StatStripProps): ReactElement {
  const minBond = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "minNominatorBond",
  });
  const activeEra = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "activeEra",
  });
  const validatorCount = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "counterForValidators",
  });

  const minBondIpt = minBond.data ? formatIpt(Number(formatEther(minBond.data as bigint))) : "...";
  const eraIndex = (activeEra.data as readonly [number, bigint] | undefined)?.[0] ?? "...";
  const validators = (validatorCount.data as number | undefined) ?? "...";

  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
      <ClayFeatureCard label="Active stake" value={formatIpt(status.active)} tone="teal">
        IPT bonded
      </ClayFeatureCard>
      <ClayFeatureCard label="Nominating" value={status.nominating.length} tone="lavender">
        {status.nominating.length === 1 ? "validator" : "validators"}
      </ClayFeatureCard>
      <ClayFeatureCard label="Active era" value={eraIndex} tone="cream">
        {validators} validators active
      </ClayFeatureCard>
      <ClayFeatureCard label="Min bond" value={minBondIpt} tone="ochre">
        IPT to nominate
      </ClayFeatureCard>
    </div>
  );
}
