"use client";

import { type ReactElement, useState } from "react";
import Link from "next/link";
import { useAccount } from "wagmi";
import { Badge } from "@artemis/coss-ui/ui/badge";
import { Button } from "@artemis/coss-ui/ui/button";
import { ClayButton, ClayHero, ClayPage } from "@artemis/coss-ui/clay";
import { cn } from "~/lib/utils";
import { CodeBlock } from "~/components/validator/CodeBlock";
import { ValidatorStatus } from "~/components/validator/ValidatorStatus";
import { SetKeysForm } from "~/components/validator/SetKeysForm";
import { ValidateForm } from "~/components/validator/ValidateForm";
import { ValidatorManage } from "~/components/validator/ValidatorManage";
import { RUN_NODE, ROTATE_KEYS, STEP_META } from "~/components/validator/content";

function ConnectNote(): ReactElement {
  return (
    <p className="text-[14px] text-[#6a6a6a]">
      Connect your validator stash wallet to complete this on-chain step.
    </p>
  );
}

function StepBody({ id, isConnected }: { id: string; isConnected: boolean }): ReactElement {
  switch (id) {
    case "run":
      return (
        <div className="flex flex-col gap-3">
          <CodeBlock code={RUN_NODE} label="bash" />
          <p className="text-[13px] text-[#6a6a6a]">
            Prefer the repo helper? Run{" "}
            <code className="rounded bg-[#f5f0e0] px-1.5 py-0.5 font-mono text-[12px]">
              SPEC=./impetus.json ROLE=validator apps/node/scripts/run-node.sh
            </code>
            .
          </p>
        </div>
      );
    case "keys":
      return <CodeBlock code={ROTATE_KEYS} label="bash" />;
    case "bond":
      return isConnected ? (
        <div className="flex flex-col gap-4">
          <ValidatorStatus />
          <Link href="/staking">
            <Button type="button" variant="outline" size="sm">
              Go to Staking →
            </Button>
          </Link>
        </div>
      ) : (
        <ConnectNote />
      );
    case "set":
      return isConnected ? <SetKeysForm /> : <ConnectNote />;
    case "validate":
      return isConnected ? <ValidateForm /> : <ConnectNote />;
    default:
      return (
        <p className="text-[15px] leading-6 text-[#6a6a6a]">
          At the next era boundary the election picks the top validators by stake. If you rank
          inside the active count you start authoring blocks and earning rewards; otherwise you wait
          in the candidate pool. Eras are roughly hourly — keep your node online and your keys
          current.
        </p>
      );
  }
}

export default function ValidatorPage(): ReactElement {
  const { isConnected } = useAccount();
  const [active, setActive] = useState(1);
  const current = STEP_META[active - 1];
  const done = active - 1;

  return (
    <ClayPage>
      <ClayHero
        eyebrow="Run a node"
        title="Become a validator"
        description="A guided setup — finish each step to join the active validator set and earn block rewards."
      />

      <div className="rounded-[1.25rem] border border-[#e8b94a] bg-[#fdf6e3] p-5">
        <p className="text-[11px] font-bold uppercase tracking-[0.1em] text-[#7a5a12]">
          Before you start
        </p>
        <ul className="mt-2 flex list-disc flex-col gap-1.5 pl-5 text-[13.5px] leading-6 text-[#3a3a3a]">
          <li>
            Impetus is a <strong>canary network</strong> meshing over private networking — you need
            a reachable <strong>public bootnode</strong> from the operator to peer from outside.
          </li>
          <li>
            The active set is capped at <strong>32</strong> and elected by stake each era.
            Registering does not guarantee election.
          </li>
          <li>
            Validators can be <strong>slashed</strong> for equivocation or downtime. Run one node
            per key set and keep it online.
          </li>
        </ul>
      </div>

      <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-[300px_1fr]">
        <nav className="rounded-[20px] border border-[#ece5d6] bg-white p-2 lg:sticky lg:top-6">
          {STEP_META.map((s) => {
            const state = s.n < active ? "done" : s.n === active ? "active" : "todo";
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => setActive(s.n)}
                className={cn(
                  "flex w-full items-start gap-3 rounded-2xl px-3 py-3 text-left transition-colors",
                  state === "active" ? "bg-[#f5f0e0]" : "hover:bg-[#faf6ec]",
                )}
              >
                <span
                  className={cn(
                    "flex size-7 shrink-0 items-center justify-center rounded-full text-[13px] font-bold",
                    state === "done" && "bg-[#22c55e] text-white",
                    state === "active" && "bg-[#0a0a0a] text-white",
                    state === "todo" && "bg-[#f5f0e0] text-[#6a6a6a]",
                  )}
                >
                  {state === "done" ? "✓" : s.n}
                </span>
                <span className="flex flex-col">
                  <span className="text-sm font-bold text-[#0a0a0a]">{s.title}</span>
                  <span className="text-[12px] text-[#6a6a6a]">{s.short}</span>
                </span>
              </button>
            );
          })}
          <p className="px-3 pb-2 pt-1 text-center font-mono text-[12px] text-[#6a6a6a]">
            {done} of {STEP_META.length} complete ·{" "}
            {Math.round((done / STEP_META.length) * 100)}%
          </p>
        </nav>

        <div className="rounded-[24px] border border-[#ece5d6] bg-white p-7">
          <Badge variant="secondary">
            Step {current.n}
            {current.onchain ? " · on-chain" : ""}
          </Badge>
          <h2 className="mt-3 text-3xl font-black leading-tight tracking-[-0.02em]">
            {current.title}
          </h2>
          <div className="mt-5">
            <StepBody id={current.id} isConnected={isConnected} />
          </div>
          <div className="mt-7 flex items-center justify-between border-t border-[#ece5d6] pt-5">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={active === 1}
              onClick={() => setActive((n) => Math.max(1, n - 1))}
            >
              ← Back
            </Button>
            <ClayButton
              type="button"
              disabled={active === STEP_META.length}
              onClick={() => setActive((n) => Math.min(STEP_META.length, n + 1))}
            >
              Next step →
            </ClayButton>
          </div>
        </div>
      </div>

      {isConnected && (
        <section className="flex flex-col gap-4 border-t border-[#ece5d6] pt-8">
          <div className="flex flex-col gap-1">
            <h2 className="text-2xl font-black tracking-[-0.02em]">Already a validator?</h2>
            <p className="max-w-2xl text-[14px] text-[#6a6a6a]">
              Update your commission, stop validating, or rotate out your session keys.
            </p>
          </div>
          <ValidatorManage />
        </section>
      )}
    </ClayPage>
  );
}
