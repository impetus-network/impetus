"use client";

import { type ReactElement } from "react";
import { useAccount } from "wagmi";
import { ClayBadge, ClayEmptyState, ClaySection } from "@artemis/coss-ui/clay";
import { PageShell } from "~/components/layout/PageShell";
import { useStakingStatus } from "~/hooks/useStakingStatus";
import { StatStrip } from "~/components/staking/StatStrip";
import { PositionPanel } from "~/components/staking/PositionPanel";
import { ValidatorPanel } from "~/components/staking/ValidatorPanel";

export default function StakingPage(): ReactElement {
  const { isConnected } = useAccount();
  const status = useStakingStatus();

  return (
    <PageShell
      eyebrow="Nominated proof-of-stake"
      title="Staking console"
      description="Bond IPT and nominate validators to help secure Impetus and earn staking rewards."
      actions={
        isConnected && (
          <ClayBadge variant={status.isBonded ? "success" : "secondary"} className="w-fit">
            {status.isBonded ? "Bonded" : "Not bonded"}
          </ClayBadge>
        )
      }
    >
      {!isConnected ? (
        <ClaySection title="Get started" description="Connect a wallet to stake IPT.">
          <ClayEmptyState
            title="Wallet not connected"
            description="Connect your wallet to bond IPT and nominate validators."
          />
        </ClaySection>
      ) : (
        <div className="flex flex-col gap-6">
          <StatStrip status={status} />
          <div className="grid grid-cols-1 items-start gap-5 lg:grid-cols-[360px_1fr]">
            <div className="lg:sticky lg:top-6">
              <PositionPanel status={status} />
            </div>
            <ValidatorPanel status={status} />
          </div>
        </div>
      )}
    </PageShell>
  );
}
