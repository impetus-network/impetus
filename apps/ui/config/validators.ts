// Known canary validator stash addresses (from apps/node/launch-keys/
// validators.json). The Staking precompile has no "list all validators" view,
// so the validators page seeds from this operator-curated set and enriches each
// entry with live on-chain reads. Add new operator stashes here as they join.
export interface KnownValidator {
  name: string;
  stash: `0x${string}`;
}

export const KNOWN_VALIDATORS: readonly KnownValidator[] = [
  { name: "validator-1", stash: "0xC9ae43471d8BE417E59185143f8e88acbE7368B1" },
  { name: "validator-2", stash: "0x5B3dc42E51C5258a5780326576453eE7a955Ac81" },
  { name: "validator-3", stash: "0x22E2eE192aB15B24F0189c2CEA744fD78F1A2191" },
  { name: "validator-4", stash: "0x47F7c81A55e0c7Aa7590E5958C28a9149ED6E05F" },
  { name: "validator-5", stash: "0x80b4B39E694c963235CD0b953331f0F4dB33C951" },
];
