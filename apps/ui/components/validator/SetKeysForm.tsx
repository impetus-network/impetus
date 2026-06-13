"use client";

import { type FormEvent, type ReactElement, useState } from "react";
import { isHex } from "viem";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { Field, FieldLabel, FieldError } from "@artemis/coss-ui/ui/field";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayButton } from "@artemis/coss-ui/clay";

// Impetus session keys = babe + grandpa + im_online + authority_discovery,
// 32 bytes each = 128 bytes => 0x + 256 hex chars.
const EXPECTED_KEYS_LENGTH = 2 + 128 * 2;

export function SetKeysForm(): ReactElement {
  const { writeAsync, isMining } = useScaffoldWriteContract("Session");
  const [keys, setKeys] = useState("");
  const [error, setError] = useState("");

  function validate(): boolean {
    setError("");
    if (!isHex(keys)) {
      setError("Keys must be a 0x-prefixed hex string from author_rotateKeys");
      return false;
    }
    if (keys.length !== EXPECTED_KEYS_LENGTH) {
      setError(`Expected 128 bytes (${EXPECTED_KEYS_LENGTH} chars), got ${keys.length}`);
      return false;
    }
    return true;
  }

  async function handleSubmit(e: FormEvent): Promise<void> {
    e.preventDefault();
    if (!validate()) return;
    try {
      // proof is unused on this runtime — pass empty bytes.
      await writeAsync("setKeys", [keys as `0x${string}`, "0x"]);
      setKeys("");
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <Field invalid={!!error}>
        <FieldLabel>Session keys (author_rotateKeys output)</FieldLabel>
        <Input
          type="text"
          placeholder="0x..."
          value={keys}
          onChange={(e) => {
            setKeys(e.target.value.trim());
            setError("");
          }}
          className="font-mono text-[13px]"
        />
        {error && <FieldError>{error}</FieldError>}
      </Field>
      <ClayButton type="submit" disabled={isMining} className="w-fit">
        {isMining ? "Submitting..." : "Register session keys"}
      </ClayButton>
    </form>
  );
}
