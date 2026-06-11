import { expect } from "chai";
import { ethers } from "hardhat";

describe("ChainlinkVRFResultSource", function () {
  async function deployWithMock() {
    const mockFactory = await ethers.getContractFactory("MockVRFCoordinator");
    const mock = await mockFactory.deploy();
    await mock.waitForDeployment();

    const keyHash = ethers.zeroPadValue("0x01", 32);
    const subscriptionId = 1n;

    const factory = await ethers.getContractFactory("ChainlinkVRFResultSource");
    const source = await factory.deploy(
      await mock.getAddress(),
      keyHash,
      subscriptionId
    );
    await source.waitForDeployment();

    return { source, mock, keyHash, subscriptionId };
  }

  describe("requestResult", function () {
    it("emits ResultRequested and stores roundId mapping", async function () {
      const { source } = await deployWithMock();
      await expect(source.requestResult(1n))
        .to.emit(source, "ResultRequested")
        .withArgs(1n);
      const requestId = await source.roundToRequest(1n);
      expect(requestId).to.be.gt(0n);
    });

    it("reverts if round already requested", async function () {
      const { source } = await deployWithMock();
      await source.requestResult(1n);
      await expect(source.requestResult(1n))
        .to.be.revertedWithCustomError(source, "RoundAlreadyRequested");
    });
  });

  describe("fulfillRandomWords", function () {
    it("generates valid lottery result with 28 numbers in range 0-99", async function () {
      const { source, mock } = await deployWithMock();
      await source.requestResult(1n);
      const requestId = await source.roundToRequest(1n);
      const word0 = ethers.toBigInt(ethers.randomBytes(32));
      const word1 = ethers.toBigInt(ethers.randomBytes(32));
      await mock.fulfillRequest(requestId, [word0, word1]);

      expect(await source.hasResult(1n)).to.be.true;
      const result = await source.getResult(1n);
      expect(result.specialPrize).to.be.lte(99);
      expect(result.allPrizes).to.have.lengthOf(27);
      expect(result.allPrizes[0]).to.equal(result.specialPrize);
      for (const prize of result.allPrizes) {
        expect(prize).to.be.lte(99);
      }
    });

    it("emits ResultFulfilled with correct roundId", async function () {
      const { source, mock } = await deployWithMock();
      await source.requestResult(5n);
      const requestId = await source.roundToRequest(5n);
      const word0 = ethers.toBigInt(ethers.randomBytes(32));
      const word1 = ethers.toBigInt(ethers.randomBytes(32));
      await expect(mock.fulfillRequest(requestId, [word0, word1]))
        .to.emit(source, "ResultFulfilled")
        .withArgs(5n);
    });
  });

  describe("getResult", function () {
    it("reverts if round not fulfilled", async function () {
      const { source } = await deployWithMock();
      await source.requestResult(1n);
      await expect(source.getResult(1n))
        .to.be.revertedWithCustomError(source, "RoundNotFulfilled");
    });

    it("reverts for unknown round", async function () {
      const { source } = await deployWithMock();
      await expect(source.getResult(999n))
        .to.be.revertedWithCustomError(source, "RoundNotFulfilled");
    });
  });

  describe("hasResult", function () {
    it("returns false before fulfill", async function () {
      const { source } = await deployWithMock();
      expect(await source.hasResult(1n)).to.be.false;
    });
  });
});
