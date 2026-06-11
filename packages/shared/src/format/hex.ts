export function formatHex(
  hex: `0x${string}`,
  head = 6,
  tail = 4,
): string {
  return `${hex.slice(0, 2 + head)}…${hex.slice(-tail)}`;
}
