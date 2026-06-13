"use client";

import { type ReactElement } from "react";
import Link from "next/link";
import { formatEther } from "viem";
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
import { ClayTableFrame } from "@artemis/coss-ui/clay";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";
import { KNOWN_VALIDATORS, type KnownValidator } from "~/config/validators";

function StatusBadge({ prefs }: { prefs: readonly [number, boolean] | undefined }): ReactElement {
  if (prefs === undefined) return <Badge variant="secondary">unknown</Badge>;
  if (prefs[1]) return <Badge variant="destructive">blocked</Badge>;
  return <Badge variant="success">active</Badge>;
}

function ValidatorRow({ validator }: { validator: KnownValidator }): ReactElement {
  const prefs = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "validators",
    args: [validator.stash],
  });
  const ledger = useScaffoldReadContract({
    contractName: "Staking",
    functionName: "ledger",
    args: [validator.stash],
  });

  const prefsData = prefs.data as readonly [number, boolean] | undefined;
  const ledgerData = ledger.data as readonly [bigint, bigint, unknown] | undefined;
  const selfStake = ledgerData
    ? Number(formatEther(ledgerData[0])).toLocaleString("en-US", { maximumFractionDigits: 0 })
    : "—";

  return (
    <TableRow>
      <TableCell className="font-semibold">{validator.name}</TableCell>
      <TableCell className="font-mono text-xs text-[#6a6a6a]">
        {validator.stash.slice(0, 10)}…{validator.stash.slice(-6)}
      </TableCell>
      <TableCell className="font-mono">{prefsData ? `${prefsData[0]}%` : "—"}</TableCell>
      <TableCell className="font-mono">{selfStake} IPT</TableCell>
      <TableCell>
        <StatusBadge prefs={prefsData} />
      </TableCell>
      <TableCell>
        <Link href="/staking">
          <Button type="button" variant="outline" size="sm">
            Nominate
          </Button>
        </Link>
      </TableCell>
    </TableRow>
  );
}

export function ValidatorsTable(): ReactElement {
  return (
    <ClayTableFrame>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Validator</TableHead>
            <TableHead>Stash</TableHead>
            <TableHead>Commission</TableHead>
            <TableHead>Self stake</TableHead>
            <TableHead>Status</TableHead>
            <TableHead />
          </TableRow>
        </TableHeader>
        <TableBody>
          {KNOWN_VALIDATORS.map((v) => (
            <ValidatorRow key={v.stash} validator={v} />
          ))}
        </TableBody>
      </Table>
    </ClayTableFrame>
  );
}
