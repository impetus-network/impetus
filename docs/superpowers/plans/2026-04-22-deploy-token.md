# ERC20 Token Quick Deploy — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a token deployment flow to Artemis webapp that consumes hardhat-monorepo's existing compile + token registry API via SIWE auth.

**Architecture:** Artemis webapp calls hardhat-monorepo API directly (cross-origin). SIWE auth via better-auth client SDK. Token source code composed client-side, compiled via `/api/compile`, deployed via wagmi, registered via `POST /api/tokens`. TanStack Query for server state.

**Tech Stack:** React 19, wagmi, viem, better-auth (client + SIWE plugin), zustand (auth store), TanStack Query, siwe, React Router v7

---

## File Map

```
packages/webapp/
  src/
    features/deploy-token/
      api/
        auth-client.ts        better-auth client pointing at TOKEN_API_URL
        fetcher.ts            Authenticated fetch wrapper for token API
      lib/
        composer.ts           Solidity source code generator (port from hardhat-monorepo)
        token-registry.ts     Constructor args encoder + registry body builder
        types.ts              Shared types (TokenParams, CompileResponse, DeployedToken, DeployStep)
      stores/
        auth-store.ts         Zustand persist store for SIWE session
      hooks/
        useTokenApiAuth.ts    SIWE sign-in/out + session management
        useCompile.ts         TanStack mutation: compose + compile
        useTokenDeploy.ts     Full deploy orchestration (compile -> deploy -> register)
        useTokens.ts          TanStack query: list tokens
      components/
        DeployTokenForm.tsx   Form with token params
        CompileReview.tsx     Review compilation result + deploy button
        TokenList.tsx         Table of deployed tokens
        AuthGate.tsx          Wrapper that ensures SIWE auth before rendering children
      pages/
        DeployTokenPage.tsx   /deploy-token route
        TokenListPage.tsx     /deploy-token/list route
    App.tsx                   (modify) Add new routes
    components/layout/
      AppLayout.tsx           (modify) Add nav link
  .env.example                (modify) Add VITE_TOKEN_API_URL
  package.json                (modify) Add dependencies
```

---

### Task 1: Add Dependencies

**Files:**
- Modify: `packages/webapp/package.json`

- [ ] **Step 1: Install better-auth, siwe**

```bash
cd packages/webapp && pnpm add better-auth siwe
```

- [ ] **Step 2: Add VITE_TOKEN_API_URL to .env.example**

Append to `packages/webapp/.env.example`:

```
VITE_TOKEN_API_URL=http://localhost:3001
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/package.json packages/webapp/.env.example pnpm-lock.yaml
git commit -m "chore(webapp): add better-auth and siwe dependencies"
```

---

### Task 2: Shared Types

**Files:**
- Create: `packages/webapp/src/features/deploy-token/lib/types.ts`

- [ ] **Step 1: Create types file**

```typescript
import type { Hex } from "viem";

export type DeployStep =
  | "idle"
  | "compiling"
  | "awaiting-signature"
  | "confirming"
  | "success"
  | "error";

export interface TokenParams {
  name: string;
  symbol: string;
  initialSupply: string;
  decimals: number;
  mintable: boolean;
  burnable: boolean;
  pausable: boolean;
}

export interface CompileResult {
  abi: readonly unknown[];
  bytecode: string;
  contractName: string;
  compilerVersion: string;
  optimizerRuns: number;
  metadata: string;
}

export interface CompileApiResponse {
  data: {
    success: true;
    data: CompileResult;
  };
  status: number;
  headers: Headers;
}

export interface DeployResult {
  address: string;
  txHash: Hex;
  chainId: number;
}

export interface DeployedToken {
  id: string;
  address: string;
  chainId: number;
  name: string;
  symbol: string;
  decimals: number;
  initialSupply: string;
  mintable: boolean;
  burnable: boolean;
  pausable: boolean;
  txHash: string;
  contractName: string;
  sourceCode: string;
  compilerVersion: string;
  encodedConstructorArgs: string;
  imported: boolean;
  verificationStatus: string;
  verificationGuid: string | null;
  createdAt: string;
}

export interface TokenListResponse {
  data: {
    success: true;
    data: DeployedToken[];
  };
  status: number;
  headers: Headers;
}

export interface TokenRegistryBody {
  address: string;
  chainId: number;
  name: string;
  symbol: string;
  decimals: number;
  initialSupply: string;
  mintable: boolean;
  burnable: boolean;
  pausable: boolean;
  txHash: string;
  contractName: string;
  sourceCode: string;
  compilerVersion: string;
  encodedConstructorArgs: string;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/lib/types.ts
git commit -m "feat(webapp): add deploy-token shared types"
```

---

### Task 3: Solidity Composer

Port the contract source code generator from hardhat-monorepo.

**Files:**
- Create: `packages/webapp/src/features/deploy-token/lib/composer.ts`

- [ ] **Step 1: Create composer**

```typescript
interface TokenConfig {
  contractName?: string;
  mintable?: boolean;
  burnable?: boolean;
  pausable?: boolean;
}

export function composeContract(config: TokenConfig = {}): string {
  const {
    contractName = "Token",
    mintable = false,
    burnable = false,
    pausable = false,
  } = config;

  const imports: string[] = [
    'import "@openzeppelin/contracts/token/ERC20/ERC20.sol";',
    'import "@openzeppelin/contracts/access/Ownable.sol";',
  ];
  if (burnable) {
    imports.push(
      'import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";',
    );
  }
  if (pausable) {
    imports.push(
      'import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Pausable.sol";',
    );
  }

  const parents: string[] = ["ERC20"];
  if (burnable) parents.push("ERC20Burnable");
  if (pausable) parents.push("ERC20Pausable");
  parents.push("Ownable");

  const functions: string[] = [];

  functions.push(`
    function decimals() public view override returns (uint8) {
        return _customDecimals;
    }`);

  if (mintable) {
    functions.push(`
    function mint(address to, uint256 amount) public onlyOwner {
        _mint(to, amount);
    }`);
  }

  if (pausable) {
    functions.push(`
    function pause() public onlyOwner {
        _pause();
    }

    function unpause() public onlyOwner {
        _unpause();
    }`);

    functions.push(`
    function _update(address from, address to, uint256 value)
        internal
        override(ERC20, ERC20Pausable)
    {
        super._update(from, to, value);
    }`);
  }

  return `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

${imports.join("\n")}

contract ${contractName} is ${parents.join(", ")} {
    uint8 private _customDecimals;

    constructor(
        string memory name_,
        string memory symbol_,
        uint256 initialSupply,
        uint8 customDecimals
    ) ERC20(name_, symbol_) Ownable(msg.sender) {
        _customDecimals = customDecimals;
        _mint(msg.sender, initialSupply * 10 ** customDecimals);
    }
${functions.join("\n")}
}
`;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/lib/composer.ts
git commit -m "feat(webapp): add ERC20 solidity source composer"
```

---

### Task 4: Token Registry Utilities

**Files:**
- Create: `packages/webapp/src/features/deploy-token/lib/token-registry.ts`

- [ ] **Step 1: Create token-registry utilities**

```typescript
import { encodeAbiParameters } from "viem";
import type { TokenRegistryBody } from "./types";

interface TokenConstructorArgs {
  name: string;
  symbol: string;
  initialSupply: string;
  decimals: number;
}

export interface TokenRegistryDraft {
  address: string;
  chainId: number;
  name: string;
  symbol: string;
  decimals: number;
  initialSupply: string;
  mintable: boolean;
  burnable: boolean;
  pausable: boolean;
  txHash: string;
  contractName: string;
  sourceCode: string;
  compilerVersion: string;
  encodedConstructorArgs: string;
}

export function encodeTokenConstructorArgs(args: TokenConstructorArgs): string {
  const encoded = encodeAbiParameters(
    [
      { type: "string", name: "name_" },
      { type: "string", name: "symbol_" },
      { type: "uint256", name: "initialSupply" },
      { type: "uint8", name: "customDecimals" },
    ],
    [args.name, args.symbol, BigInt(args.initialSupply), args.decimals],
  );

  return encoded.slice(2);
}

export function toTokenRegistryBody(
  draft: TokenRegistryDraft,
): TokenRegistryBody {
  return {
    address: draft.address,
    chainId: draft.chainId,
    name: draft.name,
    symbol: draft.symbol,
    decimals: draft.decimals,
    initialSupply: draft.initialSupply,
    mintable: draft.mintable,
    burnable: draft.burnable,
    pausable: draft.pausable,
    txHash: draft.txHash,
    contractName: draft.contractName,
    sourceCode: draft.sourceCode,
    compilerVersion: draft.compilerVersion,
    encodedConstructorArgs: draft.encodedConstructorArgs,
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/lib/token-registry.ts
git commit -m "feat(webapp): add token registry utilities and constructor args encoder"
```

---

### Task 5: Auth Client + Fetcher

**Files:**
- Create: `packages/webapp/src/features/deploy-token/api/auth-client.ts`
- Create: `packages/webapp/src/features/deploy-token/api/fetcher.ts`

- [ ] **Step 1: Create better-auth client**

```typescript
import { createAuthClient } from "better-auth/client";
import { siweClient } from "better-auth/client/plugins";

const TOKEN_API_URL = import.meta.env.VITE_TOKEN_API_URL ?? "";

export const tokenAuthClient = createAuthClient({
  baseURL: TOKEN_API_URL,
  plugins: [siweClient()],
});
```

- [ ] **Step 2: Create authenticated fetcher**

```typescript
const TOKEN_API_URL = import.meta.env.VITE_TOKEN_API_URL ?? "";

export async function tokenApiFetcher<T>(
  url: string,
  options: RequestInit = {},
): Promise<T> {
  const token = localStorage.getItem("token-api-auth-token");
  const fullUrl = url.startsWith("http") ? url : `${TOKEN_API_URL}${url}`;

  const res = await fetch(fullUrl, {
    ...options,
    headers: {
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(options.headers as Record<string, string>),
    },
  });

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error ?? `Request failed (${res.status})`);
  }

  const body = await res.json();

  return {
    data: body,
    status: res.status,
    headers: res.headers,
  } as T;
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/features/deploy-token/api/
git commit -m "feat(webapp): add token API auth client and fetcher"
```

---

### Task 6: Auth Store

**Files:**
- Create: `packages/webapp/src/features/deploy-token/stores/auth-store.ts`

- [ ] **Step 1: Create zustand auth store**

```typescript
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface AuthUser {
  id: string;
  name: string;
  email: string;
}

interface TokenApiAuthStore {
  isAuthenticated: boolean;
  user: AuthUser | null;
  token: string | null;

  setAuth: (user: AuthUser, token: string) => void;
  clearAuth: () => void;
}

export const useTokenApiAuthStore = create<TokenApiAuthStore>()(
  persist(
    (set) => ({
      isAuthenticated: false,
      user: null,
      token: null,

      setAuth: (user, token) => {
        localStorage.setItem("token-api-auth-token", token);
        set({ isAuthenticated: true, user, token });
      },

      clearAuth: () => {
        localStorage.removeItem("token-api-auth-token");
        set({ isAuthenticated: false, user: null, token: null });
      },
    }),
    { name: "token-api-auth" },
  ),
);
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/stores/auth-store.ts
git commit -m "feat(webapp): add token API auth zustand store"
```

---

### Task 7: useTokenApiAuth Hook

**Files:**
- Create: `packages/webapp/src/features/deploy-token/hooks/useTokenApiAuth.ts`

- [ ] **Step 1: Create SIWE auth hook**

```typescript
import { useState, useCallback, useEffect, useRef } from "react";
import { useAccount, useSignMessage } from "wagmi";
import { SiweMessage } from "siwe";
import { tokenAuthClient } from "../api/auth-client";
import { useTokenApiAuthStore } from "../stores/auth-store";

export function useTokenApiAuth() {
  const { address, isConnected, chainId } = useAccount();
  const { signMessageAsync } = useSignMessage();

  const { isAuthenticated, user, setAuth, clearAuth } =
    useTokenApiAuthStore();
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const autoSignAttempted = useRef(false);
  const signingIn = useRef(false);
  const signInRef = useRef(() => {});

  useEffect(() => {
    if (!isConnected || !address) {
      tokenAuthClient.signOut().catch(() => {});
      clearAuth();
      autoSignAttempted.current = false;
    }
  }, [isConnected, address, clearAuth]);

  useEffect(() => {
    if (!isConnected || !address || autoSignAttempted.current) return;

    autoSignAttempted.current = true;
    let cancelled = false;

    (async () => {
      const storedToken = useTokenApiAuthStore.getState().token;
      if (storedToken) {
        try {
          const { data } = await tokenAuthClient.getSession({
            fetchOptions: {
              headers: { Authorization: `Bearer ${storedToken}` },
            },
          });
          if (cancelled) return;

          if (data?.user) return;
        } catch {
          // Session expired
        }

        if (cancelled) return;
        clearAuth();
      }

      signInRef.current();
    })();

    return () => {
      cancelled = true;
    };
  }, [isConnected, address, clearAuth]);

  const signIn = useCallback(async () => {
    if (!address || !isConnected) {
      setError("Connect wallet first");
      return;
    }

    if (signingIn.current) return;
    signingIn.current = true;

    setIsLoading(true);
    setError(null);

    const chain = chainId ?? 1;

    try {
      const { data: nonceData, error: nonceError } =
        await tokenAuthClient.siwe.nonce({
          walletAddress: address,
          chainId: chain,
        });

      if (nonceError || !nonceData) {
        throw new Error(
          nonceError?.message ?? "Failed to get nonce",
        );
      }

      const message = new SiweMessage({
        domain: window.location.host,
        address,
        statement: "Sign in to Token Deploy",
        uri: window.location.origin,
        version: "1",
        chainId: chain,
        nonce: nonceData.nonce,
      });

      const messageString = message.prepareMessage();
      const signature = await signMessageAsync({
        message: messageString,
      });

      const { data: verifyData, error: verifyError } =
        await tokenAuthClient.siwe.verify({
          message: messageString,
          signature,
          walletAddress: address,
          chainId: chain,
        });

      if (verifyError || !verifyData) {
        throw new Error(
          verifyError?.message ?? "Verification failed",
        );
      }

      const { token: sessionToken, user: verifiedUser } =
        verifyData as {
          token: string;
          user: { id: string; name: string; email: string };
        };
      setAuth(verifiedUser, sessionToken);

      setIsLoading(false);
    } catch (err: unknown) {
      const msg =
        err instanceof Error ? err.message : "Sign in failed";
      setError(
        msg.includes("User rejected") ? "Signature rejected" : msg,
      );
      setIsLoading(false);
    } finally {
      signingIn.current = false;
    }
  }, [address, isConnected, chainId, signMessageAsync, setAuth]);

  signInRef.current = signIn;

  const signOut = useCallback(async () => {
    await tokenAuthClient.signOut().catch(() => {});
    clearAuth();
  }, [clearAuth]);

  return { isAuthenticated, isLoading, error, user, signIn, signOut };
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/hooks/useTokenApiAuth.ts
git commit -m "feat(webapp): add SIWE auth hook for token API"
```

---

### Task 8: useTokenDeploy Hook

**Files:**
- Create: `packages/webapp/src/features/deploy-token/hooks/useTokenDeploy.ts`

- [ ] **Step 1: Create deploy orchestration hook**

```typescript
import { useState, useCallback, useEffect, useRef } from "react";
import {
  useAccount,
  useDeployContract,
  useWaitForTransactionReceipt,
} from "wagmi";
import type { Hex } from "viem";
import { tokenApiFetcher } from "../api/fetcher";
import { composeContract } from "../lib/composer";
import {
  encodeTokenConstructorArgs,
  toTokenRegistryBody,
  type TokenRegistryDraft,
} from "../lib/token-registry";
import type {
  TokenParams,
  CompileApiResponse,
  DeployStep,
  DeployResult,
} from "../lib/types";

interface UseTokenDeployReturn {
  step: DeployStep;
  error: string | null;
  saveError: string | null;
  result: DeployResult | null;
  deploy: (params: TokenParams) => Promise<void>;
  reset: () => void;
}

export function useTokenDeploy(): UseTokenDeployReturn {
  const [step, setStep] = useState<DeployStep>("idle");
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [result, setResult] = useState<DeployResult | null>(null);
  const [txHash, setTxHash] = useState<Hex | undefined>();

  const { address, isConnected, chainId } = useAccount();
  const { deployContractAsync } = useDeployContract();

  const confirmedRef = useRef(false);
  const tokenDraftRef = useRef<TokenRegistryDraft | null>(null);

  const { data: receipt, error: receiptError } =
    useWaitForTransactionReceipt({ hash: txHash, chainId });

  useEffect(() => {
    if (!receipt || step !== "confirming" || confirmedRef.current)
      return;
    confirmedRef.current = true;

    void (async () => {
      if (receipt.status === "reverted") {
        setError("Transaction reverted on chain.");
        setStep("error");
        return;
      }

      const contractAddress = receipt.contractAddress;
      if (!contractAddress) {
        setError(
          "Could not extract contract address from receipt.",
        );
        setStep("error");
        return;
      }

      setResult({
        address: contractAddress,
        txHash: txHash!,
        chainId: chainId!,
      });
      setStep("success");

      const tokenDraft = tokenDraftRef.current;
      if (!tokenDraft) return;

      try {
        await tokenApiFetcher("/api/tokens", {
          method: "POST",
          body: JSON.stringify(
            toTokenRegistryBody({
              ...tokenDraft,
              address: contractAddress,
              txHash: txHash!,
              chainId: chainId!,
            }),
          ),
        });
        setSaveError(null);
      } catch (saveErr: unknown) {
        const message =
          saveErr instanceof Error
            ? saveErr.message
            : "Token deployed but could not be saved.";
        setSaveError(message.slice(0, 150));
      }
    })();
  }, [receipt, step, txHash, chainId]);

  const receiptFailure =
    receiptError && step === "confirming"
      ? receiptError.message.slice(0, 120)
      : null;

  const reset = useCallback(() => {
    setStep("idle");
    setError(null);
    setSaveError(null);
    setResult(null);
    setTxHash(undefined);
    tokenDraftRef.current = null;
    confirmedRef.current = false;
  }, []);

  const deploy = useCallback(
    async (params: TokenParams) => {
      if (!address || !isConnected) {
        setError("Connect your wallet first.");
        setStep("error");
        return;
      }

      if (!chainId) {
        setError("No chain selected.");
        setStep("error");
        return;
      }

      confirmedRef.current = false;
      setError(null);
      setSaveError(null);
      setResult(null);

      try {
        setStep("compiling");
        const contractName = params.symbol.trim()
          ? `${params.symbol.trim()}Token`
          : "Token";

        const sourceCode = composeContract({
          contractName,
          mintable: params.mintable,
          burnable: params.burnable,
          pausable: params.pausable,
        });

        const compiled =
          await tokenApiFetcher<CompileApiResponse>(
            "/api/compile",
            {
              method: "POST",
              body: JSON.stringify({
                sourceCode,
                contractName,
                optimizer: { enabled: true, runs: 200 },
              }),
            },
          );

        tokenDraftRef.current = {
          address:
            "0x0000000000000000000000000000000000000000",
          chainId,
          name: params.name,
          symbol: params.symbol,
          decimals: params.decimals,
          initialSupply: params.initialSupply,
          mintable: params.mintable,
          burnable: params.burnable,
          pausable: params.pausable,
          txHash:
            "0x0000000000000000000000000000000000000000000000000000000000000000",
          contractName,
          sourceCode,
          compilerVersion:
            compiled.data.data.compilerVersion,
          encodedConstructorArgs:
            encodeTokenConstructorArgs({
              name: params.name,
              symbol: params.symbol,
              initialSupply: params.initialSupply,
              decimals: params.decimals,
            }),
        };

        setStep("awaiting-signature");
        const supply = BigInt(params.initialSupply || "0");
        const hash = await deployContractAsync({
          abi: compiled.data.data
            .abi as readonly unknown[],
          bytecode: compiled.data.data.bytecode as Hex,
          args: [
            params.name,
            params.symbol,
            supply,
            params.decimals,
          ],
          chainId,
        });

        setStep("confirming");
        setTxHash(hash);
      } catch (err: unknown) {
        const message =
          err instanceof Error
            ? err.message
            : "Deployment failed";
        if (
          message.includes("User rejected") ||
          message.includes("user rejected") ||
          message.includes("ACTION_REJECTED")
        ) {
          setError("Transaction rejected.");
        } else {
          setError(message.slice(0, 150));
        }
        setStep("error");
      }
    },
    [address, isConnected, chainId, deployContractAsync],
  );

  return {
    step: receiptFailure ? "error" : step,
    error: receiptFailure ?? error,
    saveError,
    result,
    deploy,
    reset,
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/hooks/useTokenDeploy.ts
git commit -m "feat(webapp): add token deploy orchestration hook"
```

---

### Task 9: useTokens Hook

**Files:**
- Create: `packages/webapp/src/features/deploy-token/hooks/useTokens.ts`

- [ ] **Step 1: Create tokens list query hook**

```typescript
import { useQuery } from "@tanstack/react-query";
import { tokenApiFetcher } from "../api/fetcher";
import type { TokenListResponse, DeployedToken } from "../lib/types";

interface UseTokensOptions {
  chainId?: number;
  limit?: number;
  offset?: number;
  enabled?: boolean;
}

export function useTokens(options: UseTokensOptions = {}) {
  const { chainId, limit = 50, offset = 0, enabled = true } = options;

  const params = new URLSearchParams();
  if (chainId !== undefined)
    params.set("chainId", String(chainId));
  params.set("limit", String(limit));
  params.set("offset", String(offset));

  const queryString = params.toString();

  return useQuery<DeployedToken[]>({
    queryKey: ["tokens", { chainId, limit, offset }],
    queryFn: async () => {
      const response =
        await tokenApiFetcher<TokenListResponse>(
          `/api/tokens?${queryString}`,
        );
      return response.data.data;
    },
    enabled,
  });
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/hooks/useTokens.ts
git commit -m "feat(webapp): add useTokens query hook"
```

---

### Task 10: AuthGate Component

**Files:**
- Create: `packages/webapp/src/features/deploy-token/components/AuthGate.tsx`

- [ ] **Step 1: Create auth gate wrapper**

```typescript
import type { ReactNode } from "react";
import { useAccount } from "wagmi";
import { useTokenApiAuth } from "../hooks/useTokenApiAuth";
import { Button } from "@/components/ui/button";

interface AuthGateProps {
  children: ReactNode;
}

export function AuthGate({ children }: AuthGateProps) {
  const { isConnected } = useAccount();
  const { isAuthenticated, isLoading, error, signIn } =
    useTokenApiAuth();

  if (!isConnected) {
    return (
      <div className="py-12 text-center text-muted-foreground">
        Connect your wallet to deploy tokens.
      </div>
    );
  }

  if (isAuthenticated) {
    return <>{children}</>;
  }

  return (
    <div className="flex flex-col items-center gap-4 py-12">
      <p className="text-muted-foreground">
        Sign in with your wallet to access token deployment.
      </p>
      <Button onClick={signIn} disabled={isLoading}>
        {isLoading ? "Signing in..." : "Sign In"}
      </Button>
      {error && (
        <p className="text-sm text-red-500">{error}</p>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/components/AuthGate.tsx
git commit -m "feat(webapp): add AuthGate component for SIWE auth"
```

---

### Task 11: DeployTokenForm Component

**Files:**
- Create: `packages/webapp/src/features/deploy-token/components/DeployTokenForm.tsx`

- [ ] **Step 1: Create deploy form**

```typescript
import { useState } from "react";
import { Button } from "@/components/ui/button";
import type { TokenParams } from "../lib/types";

interface DeployTokenFormProps {
  onSubmit: (params: TokenParams) => void;
  disabled?: boolean;
}

export function DeployTokenForm({
  onSubmit,
  disabled = false,
}: DeployTokenFormProps) {
  const [name, setName] = useState("");
  const [symbol, setSymbol] = useState("");
  const [initialSupply, setInitialSupply] = useState("");
  const [decimals, setDecimals] = useState(18);
  const [mintable, setMintable] = useState(false);
  const [burnable, setBurnable] = useState(false);
  const [pausable, setPausable] = useState(false);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    onSubmit({
      name: name.trim(),
      symbol: symbol.trim(),
      initialSupply,
      decimals,
      mintable,
      burnable,
      pausable,
    });
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label
            htmlFor="token-name"
            className="text-sm font-medium"
          >
            Name
          </label>
          <input
            id="token-name"
            type="text"
            required
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="My Token"
            className="w-full rounded border border-border bg-background px-3 py-2 text-sm"
          />
        </div>
        <div className="space-y-1">
          <label
            htmlFor="token-symbol"
            className="text-sm font-medium"
          >
            Symbol
          </label>
          <input
            id="token-symbol"
            type="text"
            required
            value={symbol}
            onChange={(e) => setSymbol(e.target.value)}
            placeholder="MTK"
            className="w-full rounded border border-border bg-background px-3 py-2 text-sm"
          />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1">
          <label
            htmlFor="initial-supply"
            className="text-sm font-medium"
          >
            Initial Supply
          </label>
          <input
            id="initial-supply"
            type="text"
            required
            value={initialSupply}
            onChange={(e) =>
              setInitialSupply(e.target.value)
            }
            placeholder="1000000"
            className="w-full rounded border border-border bg-background px-3 py-2 text-sm"
          />
        </div>
        <div className="space-y-1">
          <label
            htmlFor="decimals"
            className="text-sm font-medium"
          >
            Decimals
          </label>
          <input
            id="decimals"
            type="number"
            min={0}
            max={18}
            value={decimals}
            onChange={(e) =>
              setDecimals(Number(e.target.value))
            }
            className="w-full rounded border border-border bg-background px-3 py-2 text-sm"
          />
        </div>
      </div>

      <div className="flex gap-6">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={mintable}
            onChange={(e) => setMintable(e.target.checked)}
          />
          Mintable
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={burnable}
            onChange={(e) => setBurnable(e.target.checked)}
          />
          Burnable
        </label>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={pausable}
            onChange={(e) => setPausable(e.target.checked)}
          />
          Pausable
        </label>
      </div>

      <Button type="submit" disabled={disabled}>
        Compile & Deploy
      </Button>
    </form>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/components/DeployTokenForm.tsx
git commit -m "feat(webapp): add DeployTokenForm component"
```

---

### Task 12: CompileReview Component

**Files:**
- Create: `packages/webapp/src/features/deploy-token/components/CompileReview.tsx`

- [ ] **Step 1: Create deploy status/review component**

```typescript
import { NavLink } from "react-router";
import { Button } from "@/components/ui/button";
import type { DeployStep, DeployResult } from "../lib/types";

interface CompileReviewProps {
  step: DeployStep;
  error: string | null;
  saveError: string | null;
  result: DeployResult | null;
  onReset: () => void;
}

const STEP_LABELS: Record<DeployStep, string> = {
  idle: "",
  compiling: "Compiling contract...",
  "awaiting-signature": "Confirm transaction in wallet...",
  confirming: "Waiting for confirmation...",
  success: "Token deployed successfully!",
  error: "Deployment failed",
};

export function CompileReview({
  step,
  error,
  saveError,
  result,
  onReset,
}: CompileReviewProps) {
  if (step === "idle") return null;

  const isProcessing =
    step === "compiling" ||
    step === "awaiting-signature" ||
    step === "confirming";

  return (
    <div className="rounded border border-border p-4 space-y-3">
      <p className="text-sm font-medium">
        {STEP_LABELS[step]}
      </p>

      {isProcessing && (
        <div className="h-1 w-full overflow-hidden rounded bg-muted">
          <div className="h-full w-1/3 animate-pulse rounded bg-primary" />
        </div>
      )}

      {step === "error" && error && (
        <div className="space-y-2">
          <p className="text-sm text-red-500">{error}</p>
          <Button variant="outline" size="sm" onClick={onReset}>
            Try Again
          </Button>
        </div>
      )}

      {step === "success" && result && (
        <div className="space-y-2">
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
            <dt className="text-muted-foreground">
              Address
            </dt>
            <dd className="font-mono break-all">
              {result.address}
            </dd>
            <dt className="text-muted-foreground">
              Tx Hash
            </dt>
            <dd className="font-mono break-all">
              {result.txHash}
            </dd>
          </dl>

          {saveError && (
            <p className="text-sm text-amber-500">
              {saveError}
            </p>
          )}

          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={onReset}
            >
              Deploy Another
            </Button>
            <NavLink to="/deploy-token/list">
              <Button variant="ghost" size="sm">
                View All Tokens
              </Button>
            </NavLink>
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/components/CompileReview.tsx
git commit -m "feat(webapp): add CompileReview deploy status component"
```

---

### Task 13: TokenList Component

**Files:**
- Create: `packages/webapp/src/features/deploy-token/components/TokenList.tsx`

- [ ] **Step 1: Create token list table**

```typescript
import { NavLink } from "react-router";
import { CHAIN_CONFIG } from "@artemis/shared";
import { useTokens } from "../hooks/useTokens";
import { useTokenApiAuthStore } from "../stores/auth-store";
import { Button } from "@/components/ui/button";

export function TokenList() {
  const { isAuthenticated } = useTokenApiAuthStore();
  const {
    data: tokens,
    isLoading,
    error,
  } = useTokens({
    chainId: CHAIN_CONFIG.chainId,
    enabled: isAuthenticated,
  });

  if (isLoading) {
    return (
      <p className="py-8 text-center text-muted-foreground">
        Loading tokens...
      </p>
    );
  }

  if (error) {
    return (
      <p className="py-8 text-center text-sm text-red-500">
        {error.message}
      </p>
    );
  }

  if (!tokens || tokens.length === 0) {
    return (
      <div className="flex flex-col items-center gap-4 py-12">
        <p className="text-muted-foreground">
          No tokens deployed yet.
        </p>
        <NavLink to="/deploy-token">
          <Button>Deploy Your First Token</Button>
        </NavLink>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border text-left text-muted-foreground">
            <th className="pb-2 pr-4 font-medium">Name</th>
            <th className="pb-2 pr-4 font-medium">
              Symbol
            </th>
            <th className="pb-2 pr-4 font-medium">
              Address
            </th>
            <th className="pb-2 pr-4 font-medium">
              Supply
            </th>
            <th className="pb-2 font-medium">Features</th>
          </tr>
        </thead>
        <tbody>
          {tokens.map((token) => (
            <tr
              key={token.id}
              className="border-b border-border"
            >
              <td className="py-2 pr-4">{token.name}</td>
              <td className="py-2 pr-4 font-mono">
                {token.symbol}
              </td>
              <td className="py-2 pr-4 font-mono text-xs">
                {token.address.slice(0, 6)}...
                {token.address.slice(-4)}
              </td>
              <td className="py-2 pr-4">
                {token.initialSupply}
              </td>
              <td className="py-2">
                <div className="flex gap-1">
                  {token.mintable && (
                    <span className="rounded bg-muted px-1.5 py-0.5 text-xs">
                      Mintable
                    </span>
                  )}
                  {token.burnable && (
                    <span className="rounded bg-muted px-1.5 py-0.5 text-xs">
                      Burnable
                    </span>
                  )}
                  {token.pausable && (
                    <span className="rounded bg-muted px-1.5 py-0.5 text-xs">
                      Pausable
                    </span>
                  )}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/webapp/src/features/deploy-token/components/TokenList.tsx
git commit -m "feat(webapp): add TokenList table component"
```

---

### Task 14: Page Components

**Files:**
- Create: `packages/webapp/src/features/deploy-token/pages/DeployTokenPage.tsx`
- Create: `packages/webapp/src/features/deploy-token/pages/TokenListPage.tsx`

- [ ] **Step 1: Create DeployTokenPage**

```typescript
import { AuthGate } from "../components/AuthGate";
import { DeployTokenForm } from "../components/DeployTokenForm";
import { CompileReview } from "../components/CompileReview";
import { useTokenDeploy } from "../hooks/useTokenDeploy";

export function DeployTokenPage() {
  const { step, error, saveError, result, deploy, reset } =
    useTokenDeploy();

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Deploy ERC-20 Token</h1>

      <AuthGate>
        <div className="space-y-6">
          <DeployTokenForm
            onSubmit={deploy}
            disabled={step !== "idle" && step !== "error"}
          />
          <CompileReview
            step={step}
            error={error}
            saveError={saveError}
            result={result}
            onReset={reset}
          />
        </div>
      </AuthGate>
    </div>
  );
}
```

- [ ] **Step 2: Create TokenListPage**

```typescript
import { AuthGate } from "../components/AuthGate";
import { TokenList } from "../components/TokenList";

export function TokenListPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Deployed Tokens</h1>

      <AuthGate>
        <TokenList />
      </AuthGate>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add packages/webapp/src/features/deploy-token/pages/
git commit -m "feat(webapp): add DeployToken and TokenList pages"
```

---

### Task 15: Routes and Navigation

**Files:**
- Modify: `packages/webapp/src/App.tsx`
- Modify: `packages/webapp/src/components/layout/AppLayout.tsx`

- [ ] **Step 1: Add routes to App.tsx**

Replace entire `App.tsx` content:

```typescript
import { Routes, Route } from "react-router";
import { AppLayout } from "@/components/layout/AppLayout";
import { Home } from "@/pages/Home";
import { Admin } from "@/pages/Admin";
import { DeployTokenPage } from "@/features/deploy-token/pages/DeployTokenPage";
import { TokenListPage } from "@/features/deploy-token/pages/TokenListPage";

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Home />} />
        <Route path="admin" element={<Admin />} />
        <Route
          path="deploy-token"
          element={<DeployTokenPage />}
        />
        <Route
          path="deploy-token/list"
          element={<TokenListPage />}
        />
      </Route>
    </Routes>
  );
}
```

- [ ] **Step 2: Add nav links to AppLayout.tsx**

In `AppLayout.tsx`, add two NavLinks after the Admin NavLink (inside the `<div className="flex items-center gap-6">` container), before the closing `</div>`:

```typescript
<NavLink
  to="/deploy-token"
  className={({ isActive }) =>
    cn("text-sm font-medium transition-colors hover:text-foreground", isActive ? "text-foreground" : "text-muted-foreground")
  }
>
  Deploy Token
</NavLink>
<NavLink
  to="/deploy-token/list"
  className={({ isActive }) =>
    cn("text-sm font-medium transition-colors hover:text-foreground", isActive ? "text-foreground" : "text-muted-foreground")
  }
>
  Tokens
</NavLink>
```

- [ ] **Step 3: Verify build compiles**

```bash
cd packages/webapp && pnpm build
```

Expected: Build succeeds with no type errors.

- [ ] **Step 4: Commit**

```bash
git add packages/webapp/src/App.tsx packages/webapp/src/components/layout/AppLayout.tsx
git commit -m "feat(webapp): add deploy-token routes and navigation"
```

---

### Task 16: Manual Smoke Test

- [ ] **Step 1: Start hardhat-monorepo services**

Ensure hardhat-monorepo compiler + API are running:

```bash
# In hardhat-monorepo
cd packages/compiler && pnpm dev  # runs on port 3000
cd packages/api && pnpm dev       # runs on port 3001
```

- [ ] **Step 2: Add VITE_TOKEN_API_URL to local .env**

In `packages/webapp/.env`, add:

```
VITE_TOKEN_API_URL=http://localhost:3001
```

- [ ] **Step 3: Ensure Artemis origin is whitelisted**

In hardhat-monorepo `.env`, ensure `FRONTEND_URL` includes Artemis webapp origin:

```
FRONTEND_URL=http://localhost:5173,http://localhost:3000
```

(Artemis webapp runs on port 3000 via Vite config — adjust if different.)

- [ ] **Step 4: Start Artemis webapp and test flow**

```bash
cd packages/webapp && pnpm dev
```

1. Open http://localhost:3000
2. Connect wallet via RainbowKit
3. Navigate to "Deploy Token"
4. Should prompt SIWE sign-in
5. Fill form: name=Test, symbol=TST, supply=1000000, decimals=18
6. Click "Compile & Deploy"
7. Confirm MetaMask transaction
8. Verify success screen shows contract address
9. Navigate to "Tokens" — verify token appears in list
