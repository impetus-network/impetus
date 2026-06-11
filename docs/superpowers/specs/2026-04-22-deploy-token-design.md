# ERC20 Token Quick Deploy

## Overview

Add a token deployment flow to Artemis webapp that consumes hardhat-monorepo's existing API and compiler service. Users connect wallet, SIWE sign-in, configure token params, compile on-the-fly, deploy via MetaMask, and register in the token registry.

## Architecture

```
Artemis Webapp (packages/webapp)
  |
  +-- Token API Client --> Hardhat-Monorepo API (auth, tokens CRUD)
  |                              |
  |                              +---> Compiler Service (compile Solidity)
  |
  +-- SIWE Auth Flow --> Hardhat-Monorepo /auth/sign-in
  |
  +-- Pages:
  |     /deploy-token       deploy form
  |     /deploy-token/list  token list
  |
  +-- Env: VITE_TOKEN_API_URL=http://localhost:<port>
```

No changes to hardhat-monorepo codebase. Only consume existing API endpoints. CORS whitelist for Artemis webapp origin required on hardhat-monorepo side.

## SIWE Auth Integration

- Artemis webapp already has RainbowKit + wagmi (wallet connected)
- On first visit to `/deploy-token`, prompt SIWE sign-in with hardhat-monorepo API
- Use `better-auth` client SDK to call hardhat-monorepo `/auth/sign-in`
- Session managed via cookie/token for subsequent API calls
- Dedicated hook `useTokenApiAuth` manages auth state
- Session expired: re-prompt sign on next action
- SIWE session is separate from Artemis webapp state, used only for token API calls

## Token Deploy Flow

1. **Form input** -- name, symbol, decimals (default 18), initialSupply, toggles: mintable/burnable/pausable
2. **Compile** -- `POST /api/compile` via hardhat-monorepo API proxy, receive ABI + bytecode
3. **Review** -- display compilation result (contract name, compiler version) + estimated gas
4. **Deploy** -- wagmi `deployContract`, MetaMask confirm, wait for tx receipt
5. **Register** -- auto-call `POST /tokens/register` with contract address, txHash, chainId (322), source code, params
6. **Done** -- success view with link to token list + contract address

### Error handling

- Compile fail: show error, do not proceed
- Deploy reject/fail: show error, user can retry
- Register fail: show warning (token already on-chain), allow retry register

## Token List Page

- Route: `/deploy-token/list`
- Call `GET /tokens` from hardhat-monorepo API (paginated)
- Default filter: `chainId=322` (Artemis)
- Table columns: name, symbol, address, supply, mintable/burnable/pausable badges, deploy date
- Click row: detail view with full token info
- Empty state: CTA link to `/deploy-token`

## Navigation

- Add "Deploy Token" link to header/nav
- `/deploy-token` -- deploy form
- `/deploy-token/list` -- token list

## File Structure

```
src/features/deploy-token/
  api/
    client.ts            API client for hardhat-monorepo (compile, tokens CRUD)
    auth.ts              better-auth client + SIWE sign-in
  hooks/
    useTokenApiAuth.ts   Auth state + sign-in trigger
    useCompile.ts        Mutation: compile contract
    useDeploy.ts         Mutation: deploy + register
    useTokens.ts         Query: list tokens
  components/
    DeployTokenForm.tsx  Form with token params
    CompileReview.tsx    Review compilation result before deploy
    TokenList.tsx        Table of deployed tokens
    TokenDetail.tsx      Token detail view
  pages/
    DeployTokenPage.tsx  /deploy-token
    TokenListPage.tsx    /deploy-token/list
```

## Dependencies

- Add `better-auth` client to packages/webapp (for SIWE auth with hardhat-monorepo)

## Env Config

```
VITE_TOKEN_API_URL=http://localhost:<port>
```

## Out of Scope

- Etherscan verification (not needed for Artemis chain)
- Changes to hardhat-monorepo codebase
- Token interaction features (transfer, balance check)
