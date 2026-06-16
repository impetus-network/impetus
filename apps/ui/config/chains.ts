import { defineChain } from "viem";

export const impetus = defineChain({
  id: 388266,
  name: "Impetus",
  nativeCurrency: { name: "Impetus Token", symbol: "IPT", decimals: 18 },
  rpcUrls: {
    default: { http: ["https://archive-sg.impetus.network"] },
  },
});
