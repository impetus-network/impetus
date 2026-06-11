import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS, getBalance } from "./helpers/setup";

// transfer(address,uint256) selector
const TRANSFER_SELECTOR = "0xa9059cbb";
// approve(address,uint256) selector
const APPROVE_SELECTOR = "0x095ea7b3";

const SUPPLY = ethers.parseEther("1000000");

describe("GaslessEdgeCases", function () {
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

  async function sudoRemoveRule(
    contractAddress: string,
    selector: string,
  ): Promise<void> {
    const keyring = new Keyring({ type: "ethereum" });
    const admin = keyring.addFromUri(DEV_ACCOUNTS.admin.privateKey);

    const selectorBytes = selector.replace("0x", "");
    const removeRuleCall = api.tx.gaslessRegistry.removeRule(
      contractAddress,
      `0x${selectorBytes}`,
    );
    const sudoCall = api.tx.sudo.sudo(removeRuleCall);

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

    // Wait for the next block so the removal is effective
    await new Promise((r) => setTimeout(r, 2000));
  }

  it("disabled rule fallback: transfer is paid when rule is disabled", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Enable rule first
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    // Disable the rule
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, false);

    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    const tx = await token.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // Gas fee charged — rule is disabled
    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });

  it("removed rule fallback: transfer is paid after rule is removed", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Set rule, then remove it
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);
    await sudoRemoveRule(tokenAddress, TRANSFER_SELECTOR);

    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    const tx = await token.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // Gas fee charged — rule was removed
    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });

  it("multiple selectors: transfer is gasless but approve is paid", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Seed Bob with tokens
    const seedTx = await token.transfer(bob.address, ethers.parseEther("1000"));
    await seedTx.wait();

    // Register ONLY transfer as gasless — NOT approve
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobToken = token.connect(bob);

    // Transfer should be gasless
    const bobBalanceBeforeTransfer = await getBalance(bob.address);
    const transferTx = await bobToken.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
    });
    const transferReceipt = await transferTx.wait();
    expect(transferReceipt?.status).to.equal(1);
    const bobBalanceAfterTransfer = await getBalance(bob.address);

    expect(bobBalanceBeforeTransfer - bobBalanceAfterTransfer).to.equal(0n);

    // Approve should be paid (not registered as gasless)
    const bobBalanceBeforeApprove = await getBalance(bob.address);
    const approveTx = await bobToken.approve(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
    });
    const approveReceipt = await approveTx.wait();
    expect(approveReceipt?.status).to.equal(1);
    const bobBalanceAfterApprove = await getBalance(bob.address);

    expect(bobBalanceBeforeApprove - bobBalanceAfterApprove).to.be.greaterThan(0n);
  });

  it("minValue fallback: transfer with value=0 is paid when minValue=1 ART", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Register with minValue = 1 ART
    const minValue = ethers.parseEther("1");
    await sudoSetRule(tokenAddress, TRANSFER_SELECTOR, minValue, true);

    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    // Transfer with no ETH value sent (value=0, below minValue threshold)
    const tx = await token.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
      value: 0n,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // Gas fee charged — value=0 is below minValue=1 ART
    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });
});
