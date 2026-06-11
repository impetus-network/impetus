import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("StakingAdmin precompile — sudo gating", function () {
  this.timeout(2 * 60 * 1000);

  it("non-sudo forceNewEra reverts NotSudo", async () => {
    const signers = await ethers.getSigners();
    const user = signers[5]; // Hardhat #5 — not the sudo key
    const admin = await ethers.getContractAt("IStakingAdmin", ADDRESSES.STAKING_ADMIN, user);
    await expect(admin.forceNewEra()).to.be.revertedWith("NotSudo");
  });

  it("non-sudo setValidatorCount reverts NotSudo", async () => {
    const signers = await ethers.getSigners();
    const user = signers[5];
    const admin = await ethers.getContractAt("IStakingAdmin", ADDRESSES.STAKING_ADMIN, user);
    await expect(admin.setValidatorCount(8)).to.be.revertedWith("NotSudo");
  });
});
