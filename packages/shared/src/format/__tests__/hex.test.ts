import { describe, expect, it } from "vitest";
import { formatHex } from "../hex";

describe("formatHex", () => {
  it("truncates long hex with ellipsis", () => {
    const hash =
      "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890" as const;
    expect(formatHex(hash)).toBe("0xabcdef…7890");
  });
  it("respects custom head and tail", () => {
    const hash =
      "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890" as const;
    expect(formatHex(hash, 4, 6)).toBe("0xabcd…567890");
  });
  it("handles short address", () => {
    const addr =
      "0x1234567890abcdef1234567890abcdef12345678" as const;
    expect(formatHex(addr)).toBe("0x123456…5678");
  });
});
