import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("FastUnstake precompile", function () {
  this.timeout(2 * 60 * 1000);

  it("erasToCheckPerBlock defaults to 0 (pallet paused at genesis)", async () => {
    const fastUnstake = await ethers.getContractAt("IFastUnstake", ADDRESSES.FAST_UNSTAKE);
    const eras = await fastUnstake.erasToCheckPerBlock();
    expect(eras).to.equal(0);
  });

  it("head() returns empty array when no batch in flight", async () => {
    const fastUnstake = await ethers.getContractAt("IFastUnstake", ADDRESSES.FAST_UNSTAKE);
    const head = await fastUnstake.head();
    expect(head.length).to.equal(0);
  });

  it("queue() returns 0 for an account not registered", async () => {
    const signers = await ethers.getSigners();
    const user = signers[5];
    const fastUnstake = await ethers.getContractAt("IFastUnstake", ADDRESSES.FAST_UNSTAKE);
    const deposit = await fastUnstake.queue(await user.getAddress());
    expect(deposit).to.equal(0n);
  });

  it("registerFastUnstake reverts CallNotAllowed when pallet is paused", async () => {
    // With ErasToCheckPerBlock = 0 (genesis default) the pallet rejects new
    // registrations. Sudo must call control(N>=1) via StakingAdmin first to
    // enable, which is covered separately by the staking-admin sudo-gating
    // spec. Here we confirm the safe genesis default surfaces a clean revert.
    const signers = await ethers.getSigners();
    const user = signers[6];
    const fastUnstake = await ethers.getContractAt("IFastUnstake", ADDRESSES.FAST_UNSTAKE, user);
    await expect(fastUnstake.registerFastUnstake()).to.be.reverted;
  });
});
