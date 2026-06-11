export function formatNumber(n: number | bigint | string): string {
  const v = typeof n === "string" ? BigInt(n) : n;
  return new Intl.NumberFormat("en-US").format(v);
}
