import type { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";

// Sudo / admin signer is supplied via env (ADMIN_PRIVATE_KEY), never committed.
// The previously hardcoded key was leaked to git history and is permanently
// compromised — do not reuse it. The Hardhat dev keys below are the public
// "test test ... junk" mnemonic accounts and are intentionally well-known.
const ADMIN_PRIVATE_KEY = process.env.ADMIN_PRIVATE_KEY;

// Public Hardhat dev keys (mnemonic "test test test test test test test test
// test test test junk") — safe to commit; only ever used against local nodes.
const HARDHAT_DEV_KEYS = [
  "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80", // alice / #0
  "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d", // bob   / #1
  "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a", // charlie / #2
];

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.28",
    settings: {
      evmVersion: "cancun",
    },
  },
  networks: {
    substrate: {
      url: "http://127.0.0.1:9944",
      chainId: 322644,
      // Sudo / admin (ADMIN_PRIVATE_KEY env) first, then the public Hardhat dev
      // users. If ADMIN_PRIVATE_KEY is unset, only the dev keys are wired.
      accounts: ADMIN_PRIVATE_KEY
        ? [ADMIN_PRIVATE_KEY, ...HARDHAT_DEV_KEYS]
        : HARDHAT_DEV_KEYS,
    },
    base_sepolia: {
      url: process.env.BASE_SEPOLIA_RPC_URL ?? "https://sepolia.base.org",
      chainId: 84532,
      accounts: process.env.BASE_DEPLOYER_KEY ? [process.env.BASE_DEPLOYER_KEY] : [],
    },
    impetus_dev: {
      url: "http://127.0.0.1:9944",
      chainId: 388266,
      accounts: {
        mnemonic: "test test test test test test test test test test test junk",
        count: 10,
      },
    },
  },
};

export default config;
