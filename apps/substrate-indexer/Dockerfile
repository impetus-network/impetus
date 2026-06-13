# Build requires the generated sources to be present in the context:
#   src/types  (squid-substrate-typegen — one-time, needs the chain RPC)
#   src/model  (squid-typeorm-codegen — schema only, regenerated below)
#   db/        (squid-typeorm-migration generate — needs a DB once)
# Generate src/types + db/migrations locally first (see README), then build.
FROM node:22-alpine AS build
WORKDIR /app
COPY package.json ./
RUN npm install
COPY . .
RUN npx squid-typeorm-codegen && npm run build

FROM node:22-alpine
WORKDIR /app
ENV NODE_ENV=production
COPY --from=build /app/package.json ./
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/lib ./lib
COPY --from=build /app/db ./db
COPY --from=build /app/schema.graphql ./
# Processor and GraphQL run as separate services (see docker-compose.yml);
# this image is shared by both via different `command:` overrides.
EXPOSE 4350
CMD ["node", "-r", "dotenv/config", "lib/main.js"]
