import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS, getBalance } from "./helpers/setup";

// transfer(address,uint256) selector
const TRANSFER_SELECTOR = "0xa9059cbb";
const SUPPLY = ethers.parseEther("1000000");

describe("GaslessERC20", function () {
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

  async function registerGaslessRule(
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

  it("deploys ERC20, registers transfer as gasless, verifies fee-free transfer", async function () {
    // --- Deploy ERC20 ---
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", alice);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    // Transfer some tokens to Bob so he can test gasless transfers
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const transferAmount = ethers.parseEther("1000");
    const seedTx = await token.transfer(bob.address, transferAmount);
    await seedTx.wait();

    // --- Register transfer(address,uint256) as gasless (min_value = 0) ---
    await registerGaslessRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    // --- Bob transfers tokens — should be gasless ---
    const bobToken = token.connect(bob);
    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    const tx = await bobToken.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // No native balance change — transfer is gasless (no value sent, no gas fee)
    expect(bobBalanceBefore - bobBalanceAfter).to.equal(0n);

    // Verify tokens actually moved
    const charlieTokenBalance = await token.balanceOf(charlie);
    expect(charlieTokenBalance).to.equal(ethers.parseEther("10"));
  });

  it("falls back to paid when gas limit exceeds MaxGaslessGasLimit", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();
    const tokenAddress = await token.getAddress();

    await registerGaslessRule(tokenAddress, TRANSFER_SELECTOR, 0n, true);

    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    // gasLimit 6M exceeds MaxGaslessGasLimit (5M) — should fall back to paid
    const tx = await token.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 6_000_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // Balance decreased by more than 0 — gas fee was charged
    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });

  it("falls back to paid for unregistered selectors", async function () {
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const factory = await ethers.getContractFactory("TestToken", bob);
    const token = await factory.deploy(SUPPLY);
    await token.waitForDeployment();

    // Do NOT register any rule for this contract

    const charlie = DEV_ACCOUNTS.charlie.address;
    const bobBalanceBefore = await getBalance(bob.address);

    const tx = await token.transfer(charlie, ethers.parseEther("10"), {
      gasLimit: 100_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // Gas fee charged — not registered
    expect(bobBalanceBefore - bobBalanceAfter).to.be.greaterThan(0n);
  });
});
