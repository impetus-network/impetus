"use client";

import { type FormEvent, useState } from "react";
import { isAddress, isHex } from "viem";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { Switch } from "@artemis/coss-ui/ui/switch";
import { ClayButton, ClayCard } from "@artemis/coss-ui/clay";

export function AddRuleForm() {
  const { writeAsync, isMining } = useScaffoldWriteContract("GaslessRegistry");
  const [contract, setContract] = useState("");
  const [selector, setSelector] = useState("");
  const [minValue, setMinValue] = useState("0");
  const [enabled, setEnabled] = useState(true);
  const [contractError, setContractError] = useState("");
  const [selectorError, setSelectorError] = useState("");

  function validate(): boolean {
    let valid = true;
    setContractError("");
    setSelectorError("");

    if (!contract || !isAddress(contract)) {
      setContractError("Enter a valid address");
      valid = false;
    }
    if (!selector || !isHex(selector) || selector.length !== 10) {
      setSelectorError("Enter a valid bytes4 selector (e.g. 0xa9059cbb)");
      valid = false;
    }
    return valid;
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (!validate()) return;

    try {
      await writeAsync("setRule", [
        contract as `0x${string}`,
        selector as `0x${string}`,
        BigInt(minValue || "0"),
        enabled,
      ]);
      setContract("");
      setSelector("");
      setMinValue("0");
      setEnabled(true);
    } catch {
      // Error shown via toast
    }
  }

  return (
    <ClayCard>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Field invalid={!!contractError}>
          <FieldLabel>Contract Address</FieldLabel>
          <Input
            type="text"
            placeholder="0x..."
            value={contract}
            onChange={(e) => { setContract(e.target.value); setContractError(""); }}
            className="font-mono"
          />
          {contractError && <FieldError>{contractError}</FieldError>}
        </Field>

        <Field invalid={!!selectorError}>
          <FieldLabel>Function Selector (bytes4)</FieldLabel>
          <Input
            type="text"
            placeholder="0xa9059cbb"
            value={selector}
            onChange={(e) => { setSelector(e.target.value); setSelectorError(""); }}
            className="font-mono"
          />
          {selectorError && <FieldError>{selectorError}</FieldError>}
        </Field>

        <Field>
          <FieldLabel>Min Value (wei)</FieldLabel>
          <Input
            type="text"
            inputMode="numeric"
            placeholder="0"
            value={minValue}
            onChange={(e) => setMinValue(e.target.value)}
            className="font-mono"
          />
        </Field>

        <div className="flex items-center gap-3">
          <Switch checked={enabled} onCheckedChange={setEnabled} />
          <span className="text-sm">{enabled ? "Enabled" : "Disabled"}</span>
        </div>

        <ClayButton type="submit" disabled={isMining}>
          {isMining ? "Submitting..." : "Add Rule"}
        </ClayButton>
      </form>
    </ClayCard>
  );
}
