import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra, getStakingLedger } from "./helpers/staking-helpers";

describe("Staking precompile — nominator", function () {
  this.timeout(10 * 60 * 1000);

  it("bond → nominate → wait era → check ledger", async () => {
    const signers = await ethers.getSigners();
    const nominator = signers[6]; // dev #6 (clean - #5 reserved for validator spec)
    const nominatorAddr = await nominator.getAddress();
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, nominator);

    const bondAmount = ethers.parseEther("1000");
    await (await staking.bond(bondAmount, { kind: 0, account: ethers.ZeroAddress })).wait();

    // Nominate 2 genesis validators (Hardhat #0 and #1)
    const validators = [
      "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
      "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
    ];
    await (await staking.nominate(validators)).wait();

    await waitForEra(2);

    const ledger = await getStakingLedger(nominatorAddr);
    expect(ledger.active).to.be.gte(bondAmount);
  });
});
