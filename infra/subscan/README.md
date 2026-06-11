# Subscan Essentials for Artemis

Block explorer backend for the Artemis chain (Substrate + EVM, chain ID 322).

## Architecture

```
Archive Node (sg-storage:9946)
       ↑ WebSocket
Subscan Observer → subscribes blocks, extrinsics, events
Subscan Worker   → processes ERC20/ERC721 token discovery
Subscan API      → serves HTTP API on port 4399
       ↓
PostgreSQL + Redis
```

## Prerequisites

- Docker + Docker Compose
- Network access to Artemis archive node (`100.113.228.127:9946` via Tailscale)

## Deploy

```bash
cp .env.example .env
# Edit .env if needed

docker compose up -d --build
```

First run will clone and build Subscan from source. The observer will begin indexing from block 0.

## Deploy via Dokploy

Create a Docker Compose project in Dokploy and paste the contents of `docker-compose.yml`. Set environment variables in the Dokploy UI.

Required env vars:
- `CHAIN_WS_ENDPOINT` - archive node WS (default: `ws://100.113.228.127:9946`)
- `ETH_RPC` - archive node HTTP RPC (default: `http://100.113.228.127:9946`)
- `SUBSCAN_DB_PASS` - PostgreSQL password

## API

Base URL: `http://<host>:4399`

### Core endpoints (POST, JSON body)

| Endpoint | Description |
|----------|-------------|
| `/api/scan/metadata` | Chain metadata |
| `/api/scan/blocks` | List blocks |
| `/api/scan/block` | Block details |
| `/api/scan/extrinsics` | List extrinsics |
| `/api/scan/extrinsic` | Extrinsic details |
| `/api/scan/events` | List events |
| `/api/scan/event` | Event details |

### EVM plugin endpoints (POST, JSON body)

| Endpoint | Description |
|----------|-------------|
| `/api/plugin/evm/blocks` | EVM blocks |
| `/api/plugin/evm/transactions` | EVM transactions |
| `/api/plugin/evm/transaction` | EVM tx details |
| `/api/plugin/evm/accounts` | EVM accounts |
| `/api/plugin/evm/contracts` | Contract list |
| `/api/plugin/evm/tokens` | ERC20/ERC721 tokens |
| `/api/plugin/evm/token/transfer` | Token transfers |
| `/api/plugin/evm/token/holder` | Token holders |

### Balance plugin endpoints

| Endpoint | Description |
|----------|-------------|
| `/api/plugin/balance/transfers` | Native transfers |
| `/api/plugin/balance/accounts` | Account balances |

## Where to deploy

| Server | Pros | Cons |
|--------|------|------|
| megahost (Vietnam) | ~6 GB free RAM | 30 GB free disk, validators on same host |
| sg-storage (Singapore) | 2 TB disk, archive node co-located | RAM at 98% |
| Dedicated VPS | Clean resources | Extra cost |

Recommended: **megahost** for dev/staging (chain just reset, DB small). Move to dedicated VPS for production.

## React UI

Optional frontend: [subscan-essentials-ui-react](https://github.com/subscan-explorer/subscan-essentials-ui-react)

```bash
NEXT_PUBLIC_API_HOST=http://<subscan-api-host>:4399 npm run dev
```
