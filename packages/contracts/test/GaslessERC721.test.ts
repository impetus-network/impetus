import { expect } from "chai";
import { ethers } from "hardhat";
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api";
import { DEV_ACCOUNTS, getBalance } from "./helpers/setup";

// transferFrom(address,address,uint256) selector
const TRANSFER_FROM_SELECTOR = "0x23b872dd";

describe("GaslessERC721", function () {
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

  it("deploys ERC721, registers transferFrom as gasless, verifies fee-free NFT transfer", async function () {
    const alice = new ethers.Wallet(DEV_ACCOUNTS.alice.privateKey, ethers.provider);
    const bob = new ethers.Wallet(DEV_ACCOUNTS.bob.privateKey, ethers.provider);
    const charlie = DEV_ACCOUNTS.charlie.address;

    // --- Deploy TestNFT ---
    const factory = await ethers.getContractFactory("TestNFT", alice);
    const nft = await factory.deploy();
    await nft.waitForDeployment();
    const nftAddress = await nft.getAddress();

    // Mint token #1 to Bob
    const mintTx = await nft.mint(bob.address, 1n);
    await mintTx.wait();

    // Verify Bob owns token #1
    expect(await nft.ownerOf(1n)).to.equal(bob.address);

    // --- Register transferFrom(address,address,uint256) as gasless (min_value = 0) ---
    await registerGaslessRule(nftAddress, TRANSFER_FROM_SELECTOR, 0n, true);

    // --- Bob approves himself (needed for ERC721 transferFrom with self as operator) ---
    // Bob transfers NFT #1 to Charlie using transferFrom — should be gasless
    const bobNft = nft.connect(bob);

    // Bob approves self or uses transferFrom directly (owner can call transferFrom)
    const bobBalanceBefore = await getBalance(bob.address);

    const tx = await bobNft.transferFrom(bob.address, charlie, 1n, {
      gasLimit: 100_000,
    });
    const receipt = await tx.wait();
    expect(receipt?.status).to.equal(1);

    const bobBalanceAfter = await getBalance(bob.address);

    // No native balance change — transfer is gasless (no value sent, no gas fee)
    expect(bobBalanceBefore - bobBalanceAfter).to.equal(0n);

    // Verify Charlie now owns token #1
    expect(await nft.ownerOf(1n)).to.equal(charlie);
  });
});
