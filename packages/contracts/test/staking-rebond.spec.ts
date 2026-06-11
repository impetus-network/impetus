import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, getStakingLedger } from "./helpers/staking-helpers";

describe("Staking precompile — rebond before withdraw", function () {
  this.timeout(5 * 60 * 1000);

  it("bond → unbond → rebond cancels the unbonding chunk", async () => {
    const signers = await ethers.getSigners();
    const bonder = signers[7]; // dev #7 (clean - #6 used by nominator spec)
    const bonderAddr = await bonder.getAddress();
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, bonder);

    const bondAmount = ethers.parseEther("1000");
    await (await staking.bond(bondAmount, { kind: 0, account: ethers.ZeroAddress })).wait();
    await (await staking.unbond(ethers.parseEther("500"))).wait();

    const midLedger = await getStakingLedger(bonderAddr);
    expect(midLedger.unlocking.length).to.equal(1);

    await (await staking.rebond(ethers.parseEther("500"))).wait();

    const finalLedger = await getStakingLedger(bonderAddr);
    expect(finalLedger.unlocking.length).to.equal(0);
    expect(finalLedger.active).to.equal(bondAmount);
  });
});
