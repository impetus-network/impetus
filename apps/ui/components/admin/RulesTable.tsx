"use client";

import { type GaslessRuleRow } from "~/hooks/useGaslessRules";
import { useScaffoldWriteContract } from "~/hooks/useScaffoldWriteContract";
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
import { ClayEmptyState, ClayTableFrame } from "@artemis/coss-ui/clay";
import { formatEther } from "viem";

interface RulesTableProps {
  rules: GaslessRuleRow[];
  isAdmin: boolean;
}

export function RulesTable({ rules, isAdmin }: RulesTableProps) {
  const { writeAsync, isMining } = useScaffoldWriteContract("GaslessRegistry");

  async function handleToggle(rule: GaslessRuleRow) {
    await writeAsync("setRule", [
      rule.contract as `0x${string}`,
      rule.selector as `0x${string}`,
      BigInt(rule.minValue),
      !rule.enabled,
    ]);
  }

  async function handleRemove(rule: GaslessRuleRow) {
    await writeAsync("removeRule", [
      rule.contract as `0x${string}`,
      rule.selector as `0x${string}`,
    ]);
  }

  if (rules.length === 0) {
    return (
      <ClayEmptyState
        title="No gasless rules configured"
        description="Rules created by the sudo account will appear here."
      />
    );
  }

  return (
    <ClayTableFrame>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Contract</TableHead>
            <TableHead>Selector</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Min Value</TableHead>
            {isAdmin && <TableHead>Actions</TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rules.map((rule) => (
            <TableRow key={rule.id}>
              <TableCell className="font-mono text-xs">
                {rule.contract.slice(0, 10)}...{rule.contract.slice(-6)}
              </TableCell>
              <TableCell className="font-mono text-xs">{rule.selector}</TableCell>
              <TableCell>
                <Badge variant={rule.enabled ? "success" : "secondary"}>
                  {rule.enabled ? "Enabled" : "Disabled"}
                </Badge>
              </TableCell>
              <TableCell className="font-mono text-xs">
                {BigInt(rule.minValue) === 0n ? "0" : formatEther(BigInt(rule.minValue))}
              </TableCell>
              {isAdmin && (
                <TableCell>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={isMining}
                      onClick={() => handleToggle(rule)}
                    >
                      {rule.enabled ? "Disable" : "Enable"}
                    </Button>
                    <Button
                      variant="destructive"
                      size="sm"
                      disabled={isMining}
                      onClick={() => handleRemove(rule)}
                    >
                      Remove
                    </Button>
                  </div>
                </TableCell>
              )}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </ClayTableFrame>
  );
}
