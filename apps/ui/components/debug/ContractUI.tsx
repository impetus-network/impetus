"use client";

import { useState } from "react";
import type { ContractConfig } from "~/config/contracts";
import { ReadMethods } from "./ReadMethods";
import { WriteMethods } from "./WriteMethods";
import { ClayPanel } from "@artemis/coss-ui/clay";

interface ContractUIProps {
  contract: ContractConfig;
}

export function ContractUI({ contract }: ContractUIProps) {
  const [tab, setTab] = useState<"read" | "write">("read");

  return (
    <ClayPanel className="p-0">
      <div className="flex flex-col gap-4 border-b border-[#0a0a0a] px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <h3 className="clay-ink text-xl font-black leading-tight">
            {contract.name}
          </h3>
          <p className="mt-1 break-all font-mono text-xs font-medium text-[#6a6a6a]">
            {contract.address}
          </p>
        </div>
        <div className="clay-border flex w-fit rounded-full border bg-white p-1">
          <button
            type="button"
            onClick={() => setTab("read")}
            className={`rounded-full px-4 py-2 text-xs font-black transition-colors ${
              tab === "read" ? "bg-[#ffb084] text-[#0a0a0a]" : "text-[#6a6a6a]"
            }`}
          >
            Read
          </button>
          <button
            type="button"
            onClick={() => setTab("write")}
            className={`rounded-full px-4 py-2 text-xs font-black transition-colors ${
              tab === "write" ? "bg-[#ff4d8b] text-white" : "text-[#6a6a6a]"
            }`}
          >
            Write
          </button>
        </div>
      </div>
      <div className="p-5">
        {tab === "read" && <ReadMethods address={contract.address} abi={contract.abi} />}
        {tab === "write" && <WriteMethods address={contract.address} abi={contract.abi} />}
      </div>
    </ClayPanel>
  );
}
