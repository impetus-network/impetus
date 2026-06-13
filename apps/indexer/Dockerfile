# Ponder indexer for Impetus mainnet (chain id 388266).
# Runtime env (set on the deploy platform):
#   PONDER_RPC_URL_388266  node RPC the indexer can reach (archive/full, not the proxy)
#   DATABASE_URL           Postgres connection string
# Serves the GraphQL API on :42069 (consumed by the dApp via NEXT_PUBLIC_PONDER_URL).
FROM node:22-alpine AS deps
RUN corepack enable && corepack prepare pnpm@latest --activate
WORKDIR /app

COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./
COPY apps/indexer/package.json ./apps/indexer/
COPY packages/shared/package.json ./packages/shared/

RUN pnpm install --frozen-lockfile

FROM node:22-alpine AS runner
RUN corepack enable && corepack prepare pnpm@latest --activate
WORKDIR /app
ENV NODE_ENV=production

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/apps/indexer/node_modules ./apps/indexer/node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules
COPY packages/shared ./packages/shared
COPY apps/indexer ./apps/indexer
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./

WORKDIR /app/apps/indexer
EXPOSE 42069
# `ponder start` runs the indexer and serves GraphQL; it auto-runs codegen.
CMD ["pnpm", "start"]
