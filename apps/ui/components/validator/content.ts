// Shared copy for the validator guide, consumed by the live page and the
// layout preview variants so the steps stay identical across designs.

export const RUN_NODE = `# 1. Fetch the raw chain spec (must be byte-identical to the network, or you fork).
#    Genesis hash: 0x4fa577677170302035f28d3291adaae262353854d4e4dbb469fa5cce2c29daeb
curl -fsSL "https://gist.githubusercontent.com/DuanTranHuy/7390587912606cdad5a3d329f74f7591/raw/impetus.json" -o impetus.json

# 2. Run a synced validator node (Docker). RPC is bound to localhost only
#    because --rpc-methods unsafe is required for the next step.
docker run -d --name impetus-validator --restart unless-stopped \\
  -p 127.0.0.1:9944:9944 -p 30333:30333 \\
  -v impetus-data:/data \\
  -v "$PWD/impetus.json:/spec.json:ro" \\
  --entrypoint impetus-node tranhuyduan/impetus-railway:v0.3.0 \\
  --base-path /data --chain /spec.json --name "my-validator" \\
  --validator --sync warp --rpc-methods unsafe --rpc-cors all \\
  --bootnodes "/ip4/96.9.228.200/tcp/30333/p2p/12D3KooWPH4Dp9n9csjEgWy3MWsnrRfEy1ewCYkahG59uNt5S3q9 /ip4/116.118.47.48/tcp/30333/p2p/12D3KooWDLzWZxgqwtab7mH2YLb7NWEgpNdkumHmJRySnmEgBjEL /ip4/157.10.198.51/tcp/30333/p2p/12D3KooWATCQCifMZqrAabzLsSa5YQEWPDRw3fVpCukKvPR1swZB"

# Watch it sync to the finalized head:
docker logs -f impetus-validator`;

export const ROTATE_KEYS = `# On the node host, ask the running node to generate a fresh set of
# session keys in its keystore and return the public bundle.
curl -s -H 'Content-Type: application/json' \\
  -d '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \\
  http://127.0.0.1:9944

# => {"jsonrpc":"2.0","result":"0x<256 hex chars>","id":1}
# Copy the "result" 0x... value — these are your session keys for step 4.`;

export interface StepMeta {
  id: string;
  n: number;
  title: string;
  short: string;
  onchain: boolean;
}

export const STEP_META: readonly StepMeta[] = [
  { id: "run", n: 1, title: "Run & sync a node", short: "Server · off-chain", onchain: false },
  { id: "keys", n: 2, title: "Generate session keys", short: "author_rotateKeys", onchain: false },
  { id: "bond", n: 3, title: "Bond your stake", short: "≥ 1,000 IPT", onchain: true },
  { id: "set", n: 4, title: "Register session keys", short: "Session · 0x0818", onchain: true },
  { id: "validate", n: 5, title: "Start validating", short: "Staking · 0x0810", onchain: true },
  { id: "elected", n: 6, title: "Get elected", short: "Next era", onchain: false },
];
