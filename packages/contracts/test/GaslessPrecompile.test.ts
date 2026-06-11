import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS } from "./helpers/setup";

const PRECOMPILE_ADDRESS = "0x0000000000000000000000000000000000000800";

const PRECOMPILE_ABI = [
  "function getRule(address contract_, bytes4 selector) view returns (bool enabled, uint256 minValue)",
  "function isGasless(address contract_, bytes calldata input, uint256 value, uint256 gasLimit) view returns (bool)",
  "function setRule(address contract_, bytes4 selector, uint256 minValue, bool enabled)",
  "function removeRule(address contract_, bytes4 selector)",
];

// transfer(address,uint256) selector
const TRANSFER_SELECTOR = "0xa9059cbb";

const SUPPLY = ethers.parseEther("1000000");

describe("GaslessPrecompile", function () {
  this.timeout(120_000);

  let api: ApiPromise;

  before(async function () {
    try {
      await ethers.provider.getBlockNumber();
    } catch {
      this.skip();
    }

    const provider = new WsProvider("ws://127.0.0.1:9944");
    api = await ApiPromise.create({ provider });
  });

  after(async function () {
    if (api) {
      await api.disconnect();
    }
  });

  async function sudoSetRule(
    contractAddress: string,
    selector: string,
    minValue: bigint,
    enabled: boolean,
  ): Promise<void> {
    const keyring = new Keyring({ type: "ethereum" });
    const admin = keyring.addFromUri(DEV_ACCOUNTS.admin.privateKey);

    const selectorBytes = selector.replace("0x", "");
    const setRuleCall = api.tx.gaslessRegistry.setRule(
      contractAddress,
      `0x${selectorBytes}`,
      minValue,
      enabled,
    );
    const sudoCall = api.tx.sudo.sudo(setRuleCall);

    await new Promise<void>((resolve, reject) => {
      sudoCall.signAndSend(admin, ({ status, dispatchError }) => {
        if (dispatchError) {
          reject(new Error(dispatchError.toString()));
        }
        if (status.isInBlock || status.isFinalized) {
          resolve();
        }
      });
    });

    // Wait for the next block so the rule is active for EVM calls
    await new Promise((r) => setTimeout(r, 2000));
  }

  it("getRule returns stored enabled and minValue", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    const minValue = ethers.parseEther("5");
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, minValue, true);

    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, alice);
    const [enabled, storedMinValue] = await precompile.getRule(
      tokenAddress,
      TRANSFER_SELECTOR,
    );

    expect(enabled).to.equal(true);
    expect(storedMinValue).to.equal(minValue);
  });

  it("isGasless returns true for registered selector with matching conditions", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    // Build sample transfer calldata: transfer(charlie, 10 ART)
    const charlie = DEV_ACCOUNTS.charlie.address;
    const iface = new ethers.Interface(["function transfer(address to, uint256 value)"]);
    const calldata = iface.encodeFunctionData("transfer", [
      charlie,
      ethers.parseEther("10"),
    ]);

    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, alice);
    const gasless = await precompile.isGasless(
      tokenAddress,
      calldata,
      0n,
      100_000n,
    );

    expect(gasless).to.equal(true);
  });

  it("non-admin setRule reverts", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, bob);

    await expect(
      precompile.setRule(tokenAddress, TRANSFER_SELECTOR, 0n, true, {
        gasLimit: 100_000,
      }),
    ).to.be.reverted;
  });

  it("non-admin removeRule reverts", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Set a rule as admin first so there is something to remove
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    const precompile = new ethers.Contract(PRECOMPILE_ADDRESS, PRECOMPILE_ABI, bob);

    await expect(
      precompile.removeRule(tokenAddress, TRANSFER_SELECTOR, {
        gasLimit: 100_000,
      }),
    ).to.be.reverted;
  });
});
