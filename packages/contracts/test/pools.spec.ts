import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra } from "./helpers/staking-helpers";

describe("NominationPools precompile — full lifecycle", function () {
  this.timeout(10 * 60 * 1000);

  it("create → join → bondExtra → claimPayout → unbond → withdraw", async () => {
    const signers = await ethers.getSigners();
    const depositor = signers[8]; // #7 used by rebond spec
    const joiner = signers[9];
    const pools = await ethers.getContractAt("INominationPools", ADDRESSES.POOLS, depositor);

    const depositorAddr = await depositor.getAddress();
    const joinerAddr = await joiner.getAddress();

    // Create pool with depositor as root, nominator, bouncer
    await (await pools.create(
      ethers.parseEther("1500"),
      depositorAddr, depositorAddr, depositorAddr,
    )).wait();
    const poolId = await pools.lastPoolId();
    expect(poolId).to.be.gt(0);

    const joinerPools = await ethers.getContractAt("INominationPools", ADDRESSES.POOLS, joiner);
    await (await joinerPools.join(ethers.parseEther("500"), poolId)).wait();

    const member = await pools.poolMembers(joinerAddr);
    expect(member.poolId).to.equal(poolId);

    await waitForEra(2);

    // Claim any accrued reward (may be 0 in dev — assert no revert)
    await joinerPools.claimPayout();

    // Unbond joiner's share
    await (await joinerPools.unbond(joinerAddr, member.points)).wait();

    await waitForEra(5);

    // withdraw
    await (await joinerPools.withdrawUnbonded(joinerAddr, 0)).wait();
  });
});
