"use client";

import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { cookieStorage, createStorage } from "wagmi";
import { impetus } from "./chains";

const WALLET_CONNECT_PROJECT_ID =
  process.env.NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID ||
  "3a8170812b534d0ff9d794f19a901d64";

export const wagmiStorage = createStorage({
  storage: cookieStorage,
});

export function createWagmiConfig() {
  return getDefaultConfig({
    appName: "Impetus Explorer",
    projectId: WALLET_CONNECT_PROJECT_ID,
    chains: [impetus],
    ssr: true,
    storage: wagmiStorage,
  });
}
