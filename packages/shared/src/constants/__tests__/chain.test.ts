import { describe, expect, it } from "vitest";

import { CHAIN_CONFIG } from "../chain";

describe("CHAIN_CONFIG", () => {
  it("matches the Impetus mainnet chain metadata", () => {
    expect(CHAIN_CONFIG.chainId).toBe(388266);
    expect(CHAIN_CONFIG.tokenSymbol).toBe("IPT");
  });
});
