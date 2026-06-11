#!/usr/bin/env tsx
// Usage:
//   pnpm run dump-session-keys "//Alice"
//
// Emits:
//   1) JSON of the 4 public keys.
//   2) SCALE-encoded bytes for Session.sol::setKeys.
//   3) author_insertKey RPC commands per key type.

import { Keyring } from "@polkadot/keyring";
import { u8aToHex, hexToU8a } from "@polkadot/util";
import { cryptoWaitReady } from "@polkadot/util-crypto";

async function main() {
  await cryptoWaitReady();
  const seed = process.argv[2];
  if (!seed) {
    console.error("usage: dump-session-keys <seed-uri-or-mnemonic>");
    process.exit(1);
  }

  const sr25519 = new Keyring({ type: "sr25519" });
  const ed25519 = new Keyring({ type: "ed25519" });

  const babe = sr25519.addFromUri(seed);
  const grandpa = ed25519.addFromUri(seed);
  const imOnline = sr25519.addFromUri(seed);
  const authDisc = sr25519.addFromUri(seed);

  const out = {
    babe: u8aToHex(babe.publicKey),
    grandpa: u8aToHex(grandpa.publicKey),
    imOnline: u8aToHex(imOnline.publicKey),
    authorityDiscovery: u8aToHex(authDisc.publicKey),
  };
  console.log("public keys:\n" + JSON.stringify(out, null, 2));

  // SCALE-encoded SessionKeys = concat of 4x 32-byte pubkeys (sr25519=32, ed25519=32)
  const scaleBytes = new Uint8Array(128);
  scaleBytes.set(hexToU8a(out.babe), 0);
  scaleBytes.set(hexToU8a(out.grandpa), 32);
  scaleBytes.set(hexToU8a(out.imOnline), 64);
  scaleBytes.set(hexToU8a(out.authorityDiscovery), 96);
  console.log("\nsetKeys argument (hex):", u8aToHex(scaleBytes));
  console.log("setKeys proof argument (hex): 0x00 (empty proof - ownership signature is encoded elsewhere)");

  console.log("\nauthor_insertKey RPC commands:");
  for (const [keyType, seedField, pubkey] of [
    ["babe", seed, out.babe],
    ["gran", seed, out.grandpa],
    ["imon", seed, out.imOnline],
    ["audi", seed, out.authorityDiscovery],
  ] as const) {
    console.log(
      `curl -s -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"author_insertKey","params":["${keyType}","${seedField}","${pubkey}"],"id":1}' http://127.0.0.1:9944`,
    );
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
