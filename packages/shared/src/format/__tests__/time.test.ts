import { describe, expect, it } from "vitest";
import { formatRelativeTime, formatTimestamp } from "../time";

describe("formatTimestamp", () => {
  it("formats unix seconds to UTC string", () => {
    const result = formatTimestamp(1715126055);
    expect(result).toContain("UTC");
    expect(result).toContain("2024");
  });
});

describe("formatRelativeTime", () => {
  it("returns a string ending in 'ago'", () => {
    const fiveMinutesAgo = Math.floor(Date.now() / 1000) - 300;
    const result = formatRelativeTime(fiveMinutesAgo);
    expect(result).toContain("ago");
  });
});
