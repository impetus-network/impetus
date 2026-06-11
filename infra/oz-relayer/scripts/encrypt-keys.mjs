// One-shot helper: encrypt relayer private keys into Ethereum v3 keystore JSON
// files for the openzeppelin-relayer `type: local` signer.
//
// Usage:
//   KEYSTORE_PASSPHRASE='strong-pass' \
//   BASE_RELAYER_KEY=0x... \
//   ARTEMIS_RELAYER_KEY=0x... \
//   node infra/oz-relayer/scripts/encrypt-keys.mjs
//
// Output: infra/oz-relayer/keys/{base,artemis}-relayer.json
//
// Requires the ethers package available in the repo (transitively pulled in by
// @nomicfoundation/hardhat-toolbox in packages/contracts).

import { Wallet } from "ethers";
import { writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const keysDir = resolve(here, "..", "keys");

const passphrase = process.env.KEYSTORE_PASSPHRASE;
if (!passphrase) {
  console.error("KEYSTORE_PASSPHRASE env var is required");
  process.exit(1);
}

const targets = [
  { name: "base-relayer", env: "BASE_RELAYER_KEY" },
  { name: "artemis-relayer", env: "ARTEMIS_RELAYER_KEY" },
];

await mkdir(keysDir, { recursive: true });

for (const { name, env } of targets) {
  const pk = process.env[env];
  if (!pk) {
    console.error(`${env} not set, skipping ${name}`);
    continue;
  }
  const wallet = new Wallet(pk);
  const json = await wallet.encrypt(passphrase);
  const path = resolve(keysDir, `${name}.json`);
  await writeFile(path, json, "utf8");
  console.log(`wrote ${path} (address ${wallet.address})`);
}
