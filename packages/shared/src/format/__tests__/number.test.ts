import { describe, expect, it } from "vitest";
import { formatNumber } from "../number";

describe("formatNumber", () => {
  it("formats number with commas", () => {
    expect(formatNumber(1234567)).toBe("1,234,567");
  });
  it("formats bigint", () => {
    expect(formatNumber(9876543210n)).toBe("9,876,543,210");
  });
  it("formats string number", () => {
    expect(formatNumber("42000")).toBe("42,000");
  });
});
