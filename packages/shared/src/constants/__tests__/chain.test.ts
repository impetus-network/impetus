import { describe, expect, it } from "vitest";

import { CHAIN_CONFIG } from "../chain";

describe("CHAIN_CONFIG", () => {
  it("matches the Impulse dev/testnet chain metadata", () => {
    expect(CHAIN_CONFIG.chainId).toBe(322644);
    expect(CHAIN_CONFIG.tokenSymbol).toBe("IPL");
  });
});
