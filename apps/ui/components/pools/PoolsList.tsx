"use client";

import { type ReactElement } from "react";
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
import { ClayEmptyState, ClayTableFrame } from "@artemis/coss-ui/clay";
import { POOL_STATE_LABELS } from "@artemis/shared";
import { useScaffoldReadContract } from "~/hooks/useScaffoldReadContract";

const MAX_POOLS_SHOWN = 50;

function PoolRow({ poolId }: { poolId: number }): ReactElement | null {
  const pool = useScaffoldReadContract({
    contractName: "NominationPools",
    functionName: "bondedPools",
    args: [poolId],
  });

  const data = pool.data as
    | readonly [bigint, number, number, readonly string[], readonly [number, number, unknown, string]]
    | undefined;
  if (data === undefined) return null;

  const [points, state, memberCounter, , commission] = data;
  const stateLabel = POOL_STATE_LABELS[state] ?? "Unknown";

  return (
    <TableRow>
      <TableCell className="font-mono font-semibold">#{poolId}</TableCell>
      <TableCell className="font-mono">
        {Number(formatEther(points)).toLocaleString("en-US", { maximumFractionDigits: 0 })} IPT
      </TableCell>
      <TableCell className="font-mono">{memberCounter}</TableCell>
      <TableCell className="font-mono">{(commission[0] / 10_000_000).toFixed(1)}%</TableCell>
      <TableCell>
        <Badge variant={state === 0 ? "success" : "secondary"}>{stateLabel}</Badge>
      </TableCell>
    </TableRow>
  );
}

export function PoolsList(): ReactElement {
  const last = useScaffoldReadContract({
    contractName: "NominationPools",
    functionName: "lastPoolId",
  });

  const count = (last.data as number | undefined) ?? 0;
  if (count === 0) {
    return (
      <ClayEmptyState
        title="No pools yet"
        description="No nomination pools have been created. Create the first one below."
      />
    );
  }

  const ids = Array.from({ length: Math.min(count, MAX_POOLS_SHOWN) }, (_, i) => i + 1);

  return (
    <ClayTableFrame>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Pool</TableHead>
            <TableHead>Bonded</TableHead>
            <TableHead>Members</TableHead>
            <TableHead>Commission</TableHead>
            <TableHead>State</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {ids.map((id) => (
            <PoolRow key={id} poolId={id} />
          ))}
        </TableBody>
      </Table>
    </ClayTableFrame>
  );
}
