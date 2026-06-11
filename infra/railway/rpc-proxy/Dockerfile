# Public RPC gateway (Caddy) for Impetus. Enforces a caller allow-list on
# *.impetus.network and rate-limits, then proxies HTTP JSON-RPC + WebSocket to
# the internal pruned RPC node. Build context = this directory.
#
# The standard Caddy image lacks rate limiting, so build a custom binary with
# the caddy-ratelimit module via xcaddy.

FROM caddy:2-builder AS builder
RUN xcaddy build --with github.com/mholt/caddy-ratelimit

FROM caddy:2-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
COPY Caddyfile /etc/caddy/Caddyfile

# Railway injects $PORT and terminates TLS at its edge; Caddy serves plain HTTP.
EXPOSE 8080
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile", "--adapter", "caddyfile"]
