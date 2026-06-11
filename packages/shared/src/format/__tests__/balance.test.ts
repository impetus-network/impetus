import { describe, expect, it } from "vitest";
import { formatBalance } from "../balance";

describe("formatBalance", () => {
  it("formats wei string to human-readable with default 4 decimals", () => {
    expect(formatBalance("1234500000000000000", 18)).toBe("1.2345");
  });
  it("formats bigint input", () => {
    expect(formatBalance(1234500000000000000n, 18)).toBe("1.2345");
  });
  it("groups thousands with commas", () => {
    expect(formatBalance("123456789000000000000000", 18)).toBe("123,456.789");
  });
  it("trims trailing zeros from fractional part", () => {
    expect(formatBalance("1000000000000000000", 18)).toBe("1");
  });
  it("respects displayDecimals parameter", () => {
    expect(formatBalance("1234567890000000000", 18, 2)).toBe("1.23");
  });
  it("formats zero", () => {
    expect(formatBalance("0", 18)).toBe("0");
  });
  it("formats non-18-decimal tokens", () => {
    expect(formatBalance("1500000", 6, 2)).toBe("1.5");
  });
});
