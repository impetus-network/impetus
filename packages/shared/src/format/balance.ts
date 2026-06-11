import { formatUnits } from "viem";

export function formatBalance(
  wei: bigint | string,
  decimals: number,
  displayDecimals = 4,
): string {
  const raw = formatUnits(
    typeof wei === "string" ? BigInt(wei) : wei,
    decimals,
  );
  const [int, frac = ""] = raw.split(".");
  const intGrouped = new Intl.NumberFormat("en-US").format(BigInt(int));
  const fracTrimmed = frac.slice(0, displayDecimals).replace(/0+$/, "");
  return fracTrimmed ? `${intGrouped}.${fracTrimmed}` : intGrouped;
}
