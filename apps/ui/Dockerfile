# Stage 1: Install dependencies
FROM node:22-alpine AS deps
RUN corepack enable && corepack prepare pnpm@latest --activate
WORKDIR /app

COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./
COPY apps/ui/package.json ./apps/ui/
COPY packages/shared/package.json ./packages/shared/

RUN pnpm install --frozen-lockfile

# Stage 2: Build
FROM node:22-alpine AS builder
RUN corepack enable && corepack prepare pnpm@latest --activate
RUN apk add --no-cache curl bash && \
    curl -1sLf 'https://dl.cloudsmith.io/public/infisical/infisical-cli/setup.alpine.sh' | bash && \
    apk add --no-cache infisical
WORKDIR /app

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/apps/ui/node_modules ./apps/ui/node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules

COPY packages/shared ./packages/shared
COPY apps/ui ./apps/ui
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json turbo.json ./

ARG INFISICAL_CLIENT_ID
ARG INFISICAL_CLIENT_SECRET
ARG INFISICAL_PROJECT_ID

RUN pnpm --filter @artemis/shared build 2>/dev/null || true
RUN infisical run \
    --token "$(infisical login --method=universal-auth --client-id=${INFISICAL_CLIENT_ID} --client-secret=${INFISICAL_CLIENT_SECRET} --domain https://secret-manager.impetus.network/api --silent --plain)" \
    --projectId ${INFISICAL_PROJECT_ID} \
    --env dev \
    --domain https://secret-manager.impetus.network/api \
    -- pnpm --filter @artemis/ui build

# Stage 3: Runner
FROM node:22-alpine AS runner
WORKDIR /app

RUN apk add --no-cache curl bash ca-certificates && \
    curl -1sLf 'https://dl.cloudsmith.io/public/infisical/infisical-cli/setup.alpine.sh' | bash && \
    apk add --no-cache infisical

ENV NODE_ENV=production
ENV PORT=3001

RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 nextjs

COPY --from=builder --chown=nextjs:nodejs /app/apps/ui/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/apps/ui/.next/static ./apps/ui/.next/static
COPY --chown=nextjs:nodejs apps/ui/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

USER nextjs

EXPOSE 3001

ENTRYPOINT ["/entrypoint.sh"]
CMD ["node", "apps/ui/server.js"]
