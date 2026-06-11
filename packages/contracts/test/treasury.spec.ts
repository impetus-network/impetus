import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("Treasury precompile - EVM surface", function () {
  this.timeout(5 * 60 * 1000);

  it("non-sudo spendLocal reverts NotSudo", async () => {
    const signers = await ethers.getSigners();
    const user = signers[5];
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY, user);
    await expect(
      treasury.spendLocal(ethers.parseEther("100"), await user.getAddress()),
    ).to.be.revertedWith("NotSudo");
  });

  it("treasury pot is readable", async () => {
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY);
    const pot = await treasury.pot();
    expect(pot).to.be.gte(0n);
  });

  it("payout of a nonexistent index reverts", async () => {
    const signers = await ethers.getSigners();
    const signer = signers[0];
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY, signer);
    await expect(treasury.payout(99_999_999)).to.be.reverted;
  });

  it("spendCount is readable", async () => {
    const treasury = await ethers.getContractAt("ITreasury", ADDRESSES.TREASURY);
    const count = await treasury.spendCount();
    expect(count).to.be.gte(0);
  });

  it.skip("sudo-routed spendLocal executes (requires admin private key)", async () => {
    // The genesis sudo key on impetus_dev_npos is `admin_account()` =
    // 0xd2aE0A2139dC83Cb920e3cd7B9F640922D14b872 (project admin mnemonic,
    // documented in apps/node/CLAUDE.md). It is NOT a Hardhat-derived
    // account, so we cannot sign a sudo-key transaction through Hardhat in
    // this E2E suite without separate keystore plumbing.
    //
    // The root-routed Treasury::spend_local -> beneficiary credit path is
    // covered by the runtime integration test at
    // apps/node/runtimes/impetus/tests/treasury_proposal.rs
    // (root_spend_local_credits_beneficiary_after_spend_period) which
    // bypasses Hardhat entirely. Re-enabling this E2E test requires either
    // (a) loading the admin keypair into Hardhat's accounts list, or
    // (b) submitting `Sudo::sudo(Box::new(Treasury::spend_local{..}))` via
    // polkadot.js to bypass the EVM origin restriction. Both are Plan 4
    // operational items.
  });
});
