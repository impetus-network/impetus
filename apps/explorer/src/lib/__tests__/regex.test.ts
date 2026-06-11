import { describe, expect, it } from "vitest";
import { classify } from "../regex";

describe("classify", () => {
  it("classifies pure digits as block_num", () => {
    expect(classify("12345")).toBe("block_num");
  });

  it("classifies single digit as block_num", () => {
    expect(classify("0")).toBe("block_num");
  });

  it("classifies 66-char hex as tx_hash", () => {
    expect(
      classify(
        "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      ),
    ).toBe("tx_hash");
  });

  it("classifies 42-char hex as address", () => {
    expect(classify("0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872")).toBe(
      "address",
    );
  });

  it("returns unknown for garbage", () => {
    expect(classify("hello")).toBe("unknown");
  });

  it("trims whitespace", () => {
    expect(classify("  12345  ")).toBe("block_num");
  });

  it("returns unknown for partial hex", () => {
    expect(classify("0xabcdef")).toBe("unknown");
  });

  it("returns unknown for empty string", () => {
    expect(classify("")).toBe("unknown");
  });
});
