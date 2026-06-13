"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayButton } from "@artemis/coss-ui/clay";

export function ValidateForm(): ReactElement {
  const { writeAsync, isMining } = useScaffoldWriteContract("Staking");
  const [commission, setCommission] = useState("5");
  const [error, setError] = useState("");

  function validate(): boolean {
    setError("");
    const n = Number(commission);
    if (!Number.isInteger(n) || n < 0 || n > 100) {
      setError("Commission must be a whole number between 0 and 100");
      return false;
    }
    return true;
  }

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    if (!validate()) return;
    try {
      await writeAsync("validate", [
        { commissionPercent: Number(commission), blocked: false },
      ]);
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <Field invalid={!!error}>
        <FieldLabel>Commission (%)</FieldLabel>
        <Input
          type="text"
          inputMode="numeric"
          placeholder="5"
          value={commission}
          onChange={(e) => {
            setCommission(e.target.value);
            setError("");
          }}
          className="max-w-[160px] font-mono"
        />
        {error && <FieldError>{error}</FieldError>}
      </Field>
      <p className="text-[13px] text-[#6a6a6a]">
        The percentage of staking rewards you keep before sharing the rest with your nominators.
      </p>
      <ClayButton type="submit" disabled={isMining} className="w-fit">
        {isMining ? "Submitting..." : "Start validating"}
      </ClayButton>
    </form>
  );
}
