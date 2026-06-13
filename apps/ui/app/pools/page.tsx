"use client";

import { type ReactElement } from "react";
import { useAccount } from "wagmi";
import { ClayEmptyState, ClayHero, ClayPage, ClaySection } from "@artemis/coss-ui/clay";
import { PoolMemberPanel } from "~/components/pools/PoolMemberPanel";
import { PoolsList } from "~/components/pools/PoolsList";
import { JoinPoolForm } from "~/components/pools/JoinPoolForm";
import { CreatePoolForm } from "~/components/pools/CreatePoolForm";

export default function PoolsPage(): ReactElement {
  const { isConnected } = useAccount();

  return (
    <ClayPage>
      <ClayHero
        eyebrow="Liquid staking"
        title="Nomination pools"
        description="Delegate any amount to a pool — below the nominator minimum — and the pool stakes and nominates on your behalf. Claim rewards any time."
      />

      <ClaySection title="Your position" description="Your pool membership, rewards and unbonding.">
        {isConnected ? (
          <PoolMemberPanel />
        ) : (
          <ClayEmptyState
            title="Wallet not connected"
            description="Connect your wallet to view and manage your pool position."
          />
        )}
      </ClaySection>

      <ClaySection title="Pools" description="Active nomination pools on the network.">
        <PoolsList />
      </ClaySection>

      {isConnected && (
        <div className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <ClaySection title="Join a pool" description="Delegate into an existing pool by id.">
            <JoinPoolForm />
          </ClaySection>
          <ClaySection title="Create a pool" description="Open a new pool you operate.">
            <CreatePoolForm />
          </ClaySection>
        </div>
      )}
    </ClayPage>
  );
}
