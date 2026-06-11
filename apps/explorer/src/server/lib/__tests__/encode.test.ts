import { describe, expect, it } from "vitest";
import { checksumAddr, decStr, nullable, safeNum } from "../encode";

describe("checksumAddr", () => {
  it("checksums a lowercase address", () => {
    expect(checksumAddr("0xd2ae0a2139dc83cb920e3cd7b9f640922d14b872")).toBe(
      "0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872",
    );
  });

  it("returns already-checksummed address unchanged", () => {
    const addr = "0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872";
    expect(checksumAddr(addr)).toBe(addr);
  });

  it("checksums an all-uppercase address", () => {
    expect(
      checksumAddr("0xD2AE0A2139DC83CB920E3CD7B9F640922D14B872"),
    ).toBe("0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872");
  });
});

describe("safeNum", () => {
  it("converts small bigint to number", () => {
    expect(safeNum(42n)).toBe(42);
  });

  it("converts zero", () => {
    expect(safeNum(0n)).toBe(0);
  });

  it("handles MAX_SAFE_INTEGER boundary", () => {
    expect(safeNum(BigInt(Number.MAX_SAFE_INTEGER))).toBe(
      Number.MAX_SAFE_INTEGER,
    );
  });

  it("throws on overflow", () => {
    expect(() => safeNum(BigInt(Number.MAX_SAFE_INTEGER) + 1n)).toThrow(
      "safeNum overflow",
    );
  });
});

describe("decStr", () => {
  it("converts Decimal-like to string", () => {
    const mockDecimal = { toFixed: (_dp: number) => "123456789" };
    expect(decStr(mockDecimal as never)).toBe("123456789");
  });

  it("strips fractional part via toFixed(0)", () => {
    const mockDecimal = {
      toFixed: (dp: number) => (dp === 0 ? "100" : "100.50"),
    };
    expect(decStr(mockDecimal as never)).toBe("100");
  });
});

describe("nullable", () => {
  it("returns null for null input", () => {
    expect(nullable(null, (x: number) => x * 2)).toBeNull();
  });

  it("applies fn for non-null input", () => {
    expect(nullable(5, (x) => x * 2)).toBe(10);
  });

  it("applies fn for falsy non-null input", () => {
    expect(nullable(0, (x) => x + 1)).toBe(1);
  });

  it("applies fn for empty string", () => {
    expect(nullable("", (x) => x.length)).toBe(0);
  });
});
