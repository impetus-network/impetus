import { describe, expect, it } from "vitest";
import { decodeCursor, encodeCursor } from "../cursor";

describe("cursor", () => {
  it("round-trips an object", () => {
    const original = { block_num: 12345 };
    const encoded = encodeCursor(original);
    expect(typeof encoded).toBe("string");
    const decoded = decodeCursor<{ block_num: number }>(encoded);
    expect(decoded).toEqual(original);
  });

  it("round-trips a composite cursor", () => {
    const original = { block_num: 100, tx_index: 3 };
    const encoded = encodeCursor(original);
    const decoded = decodeCursor<{ block_num: number; tx_index: number }>(
      encoded,
    );
    expect(decoded).toEqual(original);
  });

  it("produces url-safe base64", () => {
    const encoded = encodeCursor({ block_num: 999999 });
    expect(encoded).not.toMatch(/[+/=]/);
  });

  it("throws on invalid base64", () => {
    expect(() => decodeCursor("not-valid-json!!!")).toThrow();
  });
});
