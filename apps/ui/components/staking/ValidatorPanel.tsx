"use client";

import { type ReactElement, useEffect, useRef, useState } from "react";
import { type Address, getAddress, isAddress } from "viem";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { type StakingStatus } from "~/hooks/useStakingStatus";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@artemis/coss-ui/ui/table";
import { Badge } from "@artemis/coss-ui/ui/badge";
import { Button } from "@artemis/coss-ui/ui/button";
import { Input } from "@artemis/coss-ui/ui/input";
import { ClayButton } from "@artemis/coss-ui/clay";

const MAX_NOMINATIONS = 16;

// Live-verifies an entered stash against pallet_staking::Validators and shows
// its commission. There is no "list all validators" view on the precompile, so
// the table is a composer: the user assembles their nomination set by address.
function StatusCell({ value }: { value: string }): ReactElement {
  const valid = isAddress(value);
  const result = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "validators",
    args: valid ? [value as Address] : undefined,
    enabled: valid,
  });

  if (!value.trim()) return <span className="text-[12px] text-[#c9c0ac]">empty</span>;
  if (!valid) return <Badge variant="destructive">invalid address</Badge>;
  if (result.isLoading) return <span className="text-[12px] text-[#6a6a6a]">checking...</span>;

  const data = result.data as readonly [number, boolean] | undefined;
  if (result.isError || data === undefined) {
    return <Badge variant="secondary">not a validator</Badge>;
  }
  const [commission, blocked] = data;
  if (blocked) return <Badge variant="destructive">blocked</Badge>;
  return <Badge variant="success">{commission}% commission</Badge>;
}

interface ValidatorPanelProps {
  status: StakingStatus;
}

export function ValidatorPanel({ status }: ValidatorPanelProps): ReactElement {
  const { writeAsync, isMining } = useScaffoldWriteContract("Staking");
  const [rows, setRows] = useState<string[]>([""]);
  const [error, setError] = useState("");
  const seeded = useRef(false);

  useEffect(() => {
    if (!seeded.current && status.nominating.length > 0) {
      seeded.current = true;
      setRows([...status.nominating]);
    }
  }, [status.nominating]);

  function updateRow(index: number, value: string): void {
    setRows((prev) => prev.map((r, i) => (i === index ? value : r)));
    setError("");
  }
  function addRow(): void {
    setRows((prev) => (prev.length < MAX_NOMINATIONS ? [...prev, ""] : prev));
  }
  function removeRow(index: number): void {
    setRows((prev) => (prev.length > 1 ? prev.filter((_, i) => i !== index) : [""]));
  }

  async function handleNominate(): Promise<void> {
    setError("");
    const filled = rows.map((r) => r.trim()).filter((r) => r.length > 0);
    if (filled.length === 0) {
      setError("Add at least one validator address");
      return;
    }
    if (filled.some((r) => !isAddress(r))) {
      setError("One or more addresses are invalid");
      return;
    }
    const unique = Array.from(new Set(filled.map((r) => getAddress(r))));
    if (unique.length !== filled.length) {
      setError("Duplicate validator addresses");
      return;
    }
    try {
      await writeAsync("nominate", [unique]);
      status.refetch();
    } catch {
      // Error surfaced via toast
    }
  }

  return (
    <div className="overflow-hidden rounded-[1.5rem] border border-[#ece5d6] bg-white">
      <div className="flex items-center justify-between border-b border-[#ece5d6] px-5 py-3.5">
        <h3 className="text-xs font-bold uppercase tracking-[0.1em] text-[#6a6a6a]">
          Nomination set
        </h3>
        <span className="font-mono text-[11px] text-[#6a6a6a]">
          {rows.filter((r) => r.trim()).length}/{MAX_NOMINATIONS}
        </span>
      </div>

      {!status.isBonded && (
        <p className="border-b border-[#ece5d6] bg-[#fffaf0] px-5 py-3 text-[13px] text-[#6a6a6a]">
          Bond IPT first — you can only nominate once your stake is bonded.
        </p>
      )}

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Validator stash</TableHead>
            <TableHead>Status</TableHead>
            <TableHead className="w-px" />
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row, index) => (
            <TableRow key={index}>
              <TableCell>
                <Input
                  type="text"
                  placeholder="0x..."
                  value={row}
                  onChange={(e) => updateRow(index, e.target.value)}
                  className="font-mono text-[13px]"
                />
              </TableCell>
              <TableCell>
                <StatusCell value={row.trim()} />
              </TableCell>
              <TableCell>
                <Button type="button" variant="outline" size="sm" onClick={() => removeRow(index)}>
                  Remove
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>

      <div className="flex flex-col gap-3 border-t border-[#ece5d6] p-5">
        {error && <p className="text-[13px] font-medium text-[#8f1d14]">{error}</p>}
        <div className="flex items-center justify-between gap-3">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={rows.length >= MAX_NOMINATIONS}
            onClick={addRow}
          >
            + Add validator
          </Button>
          <ClayButton
            type="button"
            disabled={isMining || !status.isBonded}
            onClick={handleNominate}
            className="bg-[#ff4d8b] text-white"
          >
            {isMining ? "Submitting..." : "Nominate"}
          </ClayButton>
        </div>
      </div>
    </div>
  );
}
