FROM node:22-alpine AS base
RUN corepack enable && corepack prepare pnpm@10.8.0 --activate
WORKDIR /app

FROM base AS deps
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json turbo.json ./
COPY apps/explorer/package.json apps/explorer/
COPY packages/shared/package.json packages/shared/
COPY packages/coss-ui/package.json packages/coss-ui/
RUN pnpm install --frozen-lockfile

FROM base AS builder
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/apps/explorer/node_modules ./apps/explorer/node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules
COPY . .
RUN pnpm turbo build --filter=@artemis/explorer

FROM node:22-alpine AS runner
WORKDIR /app
RUN addgroup --system --gid 1001 nodejs && adduser --system --uid 1001 nextjs

COPY --from=builder /app/apps/explorer/.next/standalone ./
COPY --from=builder /app/apps/explorer/.next/static ./apps/explorer/.next/static
COPY --from=builder /app/apps/explorer/public ./apps/explorer/public 2>/dev/null || true

USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "apps/explorer/server.js"]
