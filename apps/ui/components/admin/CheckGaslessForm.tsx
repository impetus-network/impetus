"use client";

import { type FormEvent, useState } from "react";
import { isAddress, isHex } from "viem";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayBadge, ClayButton, ClayCard } from "@artemis/coss-ui/clay";

export function CheckGaslessForm() {
  const [contract, setContract] = useState("");
  const [calldata, setCalldata] = useState("");
  const [value, setValue] = useState("0");
  const [gasLimit, setGasLimit] = useState("21000");
  const [check, setCheck] = useState(false);
  const [contractError, setContractError] = useState("");
  const [calldataError, setCalldataError] = useState("");

  const { data: isGasless, isLoading } = useScaffoldReadContract({
    contractName: "GaslessRegistry",
    functionName: "isGasless",
    args: [
      contract as `0x${string}`,
      calldata as `0x${string}`,
      BigInt(value || "0"),
      BigInt(gasLimit || "21000"),
    ],
    enabled: check && !!contract && !!calldata,
  });

  function validate(): boolean {
    let valid = true;
    setContractError("");
    setCalldataError("");

    if (!contract || !isAddress(contract)) {
      setContractError("Enter a valid address");
      valid = false;
    }
    if (!calldata || !isHex(calldata)) {
      setCalldataError("Enter valid hex calldata");
      valid = false;
    }
    return valid;
  }

  function handleCheck(e: FormEvent) {
    e.preventDefault();
    if (!validate()) return;
    setCheck(true);
  }

  return (
    <ClayCard>
      <form onSubmit={handleCheck} className="flex flex-col gap-4">
        <Field invalid={!!contractError}>
          <FieldLabel>Contract Address</FieldLabel>
          <Input
            type="text"
            placeholder="0x..."
            value={contract}
            onChange={(e) => { setContract(e.target.value); setContractError(""); setCheck(false); }}
            className="font-mono"
          />
          {contractError && <FieldError>{contractError}</FieldError>}
        </Field>

        <Field invalid={!!calldataError}>
          <FieldLabel>Calldata (hex)</FieldLabel>
          <Input
            type="text"
            placeholder="0xa9059cbb000..."
            value={calldata}
            onChange={(e) => { setCalldata(e.target.value); setCalldataError(""); setCheck(false); }}
            className="font-mono"
          />
          {calldataError && <FieldError>{calldataError}</FieldError>}
        </Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel>Value (wei)</FieldLabel>
            <Input
              type="text"
              inputMode="numeric"
              placeholder="0"
              value={value}
              onChange={(e) => { setValue(e.target.value); setCheck(false); }}
              className="font-mono"
            />
          </Field>
          <Field>
            <FieldLabel>Gas Limit</FieldLabel>
            <Input
              type="text"
              inputMode="numeric"
              placeholder="21000"
              value={gasLimit}
              onChange={(e) => { setGasLimit(e.target.value); setCheck(false); }}
              className="font-mono"
            />
          </Field>
        </div>

        <div className="flex flex-wrap items-center gap-4">
          <ClayButton type="submit" disabled={isLoading}>
            {isLoading ? "Checking..." : "Check"}
          </ClayButton>
          {check && isGasless !== undefined && (
            <ClayBadge variant={isGasless ? "success" : "destructive"}>
              {isGasless ? "Gasless" : "Not Gasless"}
            </ClayBadge>
          )}
        </div>
      </form>
    </ClayCard>
  );
}
