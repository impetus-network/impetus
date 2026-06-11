"use client";

import { RainbowKitProvider } from "@rainbow-me/rainbowkit";
import { artemisTheme } from "~/config/rainbowTheme";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { WagmiProvider, cookieToInitialState } from "wagmi";
import { createWagmiConfig } from "~/config/wagmi";
import { ToastProvider } from "@artemis/coss-ui/ui/toast";
import { TxConfirmProvider } from "./TxConfirmProvider";
import "@rainbow-me/rainbowkit/styles.css";

interface Web3ProviderProps {
  children: React.ReactNode;
  cookie?: string | null;
}

export function Web3Provider({ children, cookie }: Web3ProviderProps) {
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) return null;

  return <Web3ProviderClient cookie={cookie}>{children}</Web3ProviderClient>;
}

function Web3ProviderClient({ children, cookie }: Web3ProviderProps) {
  const [wagmiConfig] = useState(() => createWagmiConfig());
  const [queryClient] = useState(() => new QueryClient());
  const initialState = cookieToInitialState(wagmiConfig, cookie ?? undefined);

  return (
    <WagmiProvider config={wagmiConfig} initialState={initialState}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider theme={artemisTheme}>
          <ToastProvider>
            <TxConfirmProvider>{children}</TxConfirmProvider>
          </ToastProvider>
        </RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
}
