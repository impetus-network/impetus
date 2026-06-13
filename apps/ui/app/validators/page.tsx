"use client";

import { type ReactElement } from "react";
import { ClayHero, ClayPage, ClaySection } from "@artemis/coss-ui/clay";
import { NetworkStats } from "~/components/validators/NetworkStats";
import { ValidatorsTable } from "~/components/validators/ValidatorsTable";
import { PayoutTool } from "~/components/validators/PayoutTool";

export default function ValidatorsPage(): ReactElement {
  return (
    <ClayPage>
      <ClayHero
        eyebrow="Network"
        title="Validators"
        description="The validators securing Impetus, the current network state, and era reward payouts."
      />

      <ClaySection title="Network" description="Live staking parameters from the chain.">
        <NetworkStats />
      </ClaySection>

      <ClaySection
        title="Validator set"
        description="Known operator validators with live commission and self-stake. The precompile has no list-all view, so this set is curated; new operators are added in config."
      >
        <ValidatorsTable />
      </ClaySection>

      <ClaySection
        title="Era payouts"
        description="Trigger a validator's reward for a past era. This pays every nominator backing that validator."
      >
        <PayoutTool />
      </ClaySection>
    </ClayPage>
  );
}
