"use client";

import { useAccount } from "wagmi";
import { SUDO_ADDRESS } from "@artemis/shared";
import { useGaslessRules } from "~/hooks/useGaslessRules";
import { RulesTable } from "~/components/admin/RulesTable";
import { AddRuleForm } from "~/components/admin/AddRuleForm";
import { CheckGaslessForm } from "~/components/admin/CheckGaslessForm";
import {
  ClayBadge,
  ClayEmptyState,
  ClayHero,
  ClayPage,
  ClaySection,
} from "@artemis/coss-ui/clay";

export default function GaslessManagerPage() {
  const { address, isConnected } = useAccount();
  const { data: rules, isLoading } = useGaslessRules();
  const isAdmin = isConnected && address?.toLowerCase() === SUDO_ADDRESS.toLowerCase();

  return (
    <ClayPage>
      <ClayHero
        eyebrow="Sudo controls"
        title="Gasless Manager"
        description="Manage gasless transaction rules on Impetus chain."
      >
        {isConnected && (
          <ClayBadge variant={isAdmin ? "success" : "secondary"} className="w-fit">
            {isAdmin ? "Admin" : "Read-only"}
          </ClayBadge>
        )}
      </ClayHero>

      <ClaySection
        title="Rules"
        description="Current contract selectors eligible for gasless execution."
      >
        {isLoading ? (
          <ClayEmptyState
            title="Loading rules"
            description="Fetching gasless registry state from the chain."
          />
        ) : (
          <RulesTable rules={rules ?? []} isAdmin={isAdmin} />
        )}
      </ClaySection>

      {isAdmin && (
        <ClaySection
          title="Add Rule"
          description="Create or update a selector rule from the sudo account."
        >
          <AddRuleForm />
        </ClaySection>
      )}

      <ClaySection
        title="Check Gasless"
        description="Test whether calldata matches the active registry rules."
      >
        <CheckGaslessForm />
      </ClaySection>
    </ClayPage>
  );
}
