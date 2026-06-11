import { defineChain } from "viem";

export const artemis = defineChain({
  id: 322644,
  name: "Impulse",
  nativeCurrency: { name: "Impulse Token", symbol: "IPL", decimals: 18 },
  rpcUrls: {
    default: { http: ["https://rpc.impetus.network"] },
  },
});
