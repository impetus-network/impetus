"use client";

import { debugContracts } from "~/config/contracts";
import { ContractUI } from "~/components/debug/ContractUI";
import { ClayEmptyState, ClayHero, ClayPage } from "@artemis/coss-ui/clay";

export default function DebugContracts() {
  return (
    <ClayPage>
      <ClayHero
        eyebrow="Developer console"
        title="Debug Contracts"
        description="Interact with deployed contracts on Artemis chain."
      />

      {debugContracts.length === 0 ? (
        <ClayEmptyState
          title="No contracts configured"
          description="Add contract configs before using the debugger."
        />
      ) : (
        <div className="flex flex-col gap-6">
          {debugContracts.map((contract) => (
            <ContractUI key={contract.address} contract={contract} />
          ))}
        </div>
      )}
    </ClayPage>
  );
}
