import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("BagsList precompile", function () {
  this.timeout(5 * 60 * 1000);

  it("score is non-zero for genesis validators", async () => {
    const signers = await ethers.getSigners();
    const alice = signers[0];
    const bags = await ethers.getContractAt("IBagsList", ADDRESSES.BAGS_LIST);
    const score = await bags.score(await alice.getAddress());
    expect(score).to.be.gt(0n);
  });

  it("listSize reflects bonded accounts", async () => {
    const bags = await ethers.getContractAt("IBagsList", ADDRESSES.BAGS_LIST);
    const size = await bags.listSize();
    expect(size).to.be.gte(4); // 4 genesis validators
  });
});
