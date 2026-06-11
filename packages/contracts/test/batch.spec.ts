import { expect } from "chai";
import { ethers } from "hardhat";

const BATCH = "0x0000000000000000000000000000000000000808";

const BATCH_ABI = [
  "function batchSome(address[],uint256[],bytes[],uint64[]) external",
  "function batchSomeUntilFailure(address[],uint256[],bytes[],uint64[]) external",
  "function batchAll(address[],uint256[],bytes[],uint64[]) external",
  "event SubcallSucceeded(uint256 index)",
  "event SubcallFailed(uint256 index)",
];

describe("Batch precompile (0x0808)", () => {
  let owner: any;
  let echoA: any;
  let echoB: any;
  let echoC: any;
  let batch: any;

  before(async () => {
    [owner] = await ethers.getSigners();
    const Echo = await ethers.getContractFactory("Echo");
    echoA = await Echo.deploy();
    echoB = await Echo.deploy();
    echoC = await Echo.deploy();
    await Promise.all([
      echoA.waitForDeployment(),
      echoB.waitForDeployment(),
      echoC.waitForDeployment(),
    ]);
    batch = new ethers.Contract(BATCH, BATCH_ABI, owner);
  });

  it("batchAll: three succeeds → state of all three contracts updated", async () => {
    const iface = new ethers.Interface(["function succeed(uint256)"]);
    const data = (n: number) => iface.encodeFunctionData("succeed", [n]);

    const tx = await batch.batchAll(
      [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
      [0, 0, 0],
      [data(11), data(22), data(33)],
      [0, 0, 0],
    );
    const receipt = await tx.wait();
    expect(receipt.status).to.eq(1);

    expect(await echoA.lastValue()).to.eq(11n);
    expect(await echoB.lastValue()).to.eq(22n);
    expect(await echoC.lastValue()).to.eq(33n);
  });

  it("batchAll: middle revert → tx reverts with Echo's custom error and state unchanged", async () => {
    const ifaceOk = new ethers.Interface(["function succeed(uint256)"]);
    const ifaceFail = new ethers.Interface(["function fail(uint256)"]);

    const before = await echoC.lastValue();

    await expect(
      batch.batchAll(
        [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
        [0, 0, 0],
        [
          ifaceOk.encodeFunctionData("succeed", [99]),
          ifaceFail.encodeFunctionData("fail", [42]),
          ifaceOk.encodeFunctionData("succeed", [88]),
        ],
        [0, 0, 0],
      ),
    ).to.be.revertedWithCustomError(echoB, "Boom").withArgs(42n);

    expect(await echoC.lastValue()).to.eq(before);
  });

  it("batchSome: middle revert → outer succeeds; events show 0 succeed, 1 fail, 2 succeed", async () => {
    const ifaceOk = new ethers.Interface(["function succeed(uint256)"]);
    const ifaceFail = new ethers.Interface(["function fail(uint256)"]);

    const tx = await batch.batchSome(
      [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
      [0, 0, 0],
      [
        ifaceOk.encodeFunctionData("succeed", [1]),
        ifaceFail.encodeFunctionData("fail", [9]),
        ifaceOk.encodeFunctionData("succeed", [3]),
      ],
      [50000, 50000, 50000],
      { gasLimit: 500000 },
    );
    const receipt = await tx.wait();
    expect(receipt.status).to.eq(1);

    const iface = new ethers.Interface(BATCH_ABI);
    const events = receipt.logs
      .filter((l: any) => l.address.toLowerCase() === BATCH.toLowerCase())
      .map((l: any) => iface.parseLog({ topics: l.topics, data: l.data }));

    expect(events.map((e: any) => `${e.name}(${e.args.index})`)).to.deep.eq([
      "SubcallSucceeded(0)",
      "SubcallFailed(1)",
      "SubcallSucceeded(2)",
    ]);

    expect(await echoA.lastValue()).to.eq(1n);
    expect(await echoC.lastValue()).to.eq(3n);
  });

  it("batchSomeUntilFailure: middle revert → outer succeeds; index 2 NOT executed", async () => {
    const ifaceOk = new ethers.Interface(["function succeed(uint256)"]);
    const ifaceFail = new ethers.Interface(["function fail(uint256)"]);

    const cBefore = await echoC.lastValue();

    const tx = await batch.batchSomeUntilFailure(
      [await echoA.getAddress(), await echoB.getAddress(), await echoC.getAddress()],
      [0, 0, 0],
      [
        ifaceOk.encodeFunctionData("succeed", [7]),
        ifaceFail.encodeFunctionData("fail", [0]),
        ifaceOk.encodeFunctionData("succeed", [999]),
      ],
      [50000, 50000, 50000],
      { gasLimit: 500000 },
    );
    await tx.wait();

    expect(await echoA.lastValue()).to.eq(7n);
    expect(await echoC.lastValue()).to.eq(cBefore);
  });

  it("batchAll: caller-funded sub-call value transfers debit the caller, precompile holds nothing", async () => {
    const provider = ethers.provider;
    const a = await echoA.getAddress();
    const b = await echoB.getAddress();

    const aBefore = await provider.getBalance(a);
    const bBefore = await provider.getBalance(b);
    const pBefore = await provider.getBalance(BATCH);

    const tx = await batch.batchAll(
      [a, b],
      [ethers.parseEther("1"), ethers.parseEther("2")],
      ["0x", "0x"],
      [0, 0],
      // NOTE: no msg.value — entries are non-payable now.
    );
    await tx.wait();

    expect(await provider.getBalance(a)).to.eq(aBefore + ethers.parseEther("1"));
    expect(await provider.getBalance(b)).to.eq(bBefore + ethers.parseEther("2"));
    expect(await provider.getBalance(BATCH)).to.eq(pBefore);
  });

  it("entries are non-payable — sending msg.value reverts", async () => {
    for (const fn of ["batchSome", "batchSomeUntilFailure", "batchAll"] as const) {
      await expect(
        (batch as any)[fn](
          [await echoA.getAddress()],
          [0],
          ["0x"],
          [0],
          { value: ethers.parseEther("1") },
        ),
      ).to.be.reverted;
    }
  });

  it("self-call → tx reverts in every mode", async () => {
    for (const fn of ["batchSome", "batchSomeUntilFailure", "batchAll"] as const) {
      await expect(
        (batch as any)[fn]([BATCH], [0], ["0x"], [0]),
      ).to.be.reverted;
    }
  });
});
