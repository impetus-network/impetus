import { ethers } from "hardhat";

// Sudo / admin account. The private key is supplied via ADMIN_PRIVATE_KEY (env /
// .env) and is NEVER committed — the previously hardcoded value leaked to git
// history and is permanently compromised. ADMIN_ADDRESS may pin the matching
// address for assertions; it falls back to the well-known compromised dev
// address only as a placeholder for local runs.
//
// Hardhat dev users (alice, bob, charlie) come from the standard PUBLIC mnemonic
// "test test test test test test test test test test test junk"
// (HD path m/44'/60'/0'/0/N) and are pre-funded but have no privileged role.
const ADMIN_PRIVATE_KEY = process.env.ADMIN_PRIVATE_KEY ?? "";
const ADMIN_ADDRESS =
  process.env.ADMIN_ADDRESS ?? "0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872";

export const DEV_ACCOUNTS = {
  admin: {
    address: ADMIN_ADDRESS,
    privateKey: ADMIN_PRIVATE_KEY,
  },
  // Account #0 - regular pre-funded dev user
  alice: {
    address: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    privateKey: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
  },
  // Account #1
  bob: {
    address: "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
    privateKey: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
  },
  // Account #2
  charlie: {
    address: "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
    privateKey: "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
  },
} as const;

/**
 * Create an ethers Wallet connected to the hardhat provider.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getWallet(privateKey: string): any {
  return new ethers.Wallet(privateKey, ethers.provider);
}

/**
 * Query the native balance of an address.
 */
export async function getBalance(address: string): Promise<bigint> {
  return ethers.provider.getBalance(address);
}
