import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES, waitForEra, getStakingLedger } from "./helpers/staking-helpers";

describe("Staking precompile - validator lifecycle", function () {
  this.timeout(15 * 60 * 1000);

  it("bond -> validate -> wait era -> unbond -> withdrawUnbonded", async () => {
    // signer[5] is a clean pre-funded dev account (not in the 4-validator
    // genesis set). signers[0]-signers[3] are already bonded as genesis
    // validators, so `bond` would revert with `AlreadyBonded` on them.
    //
    // setKeys is intentionally NOT exercised here: pallet-session validates
    // the proof of ownership against the keys, and we cannot generate a
    // valid signature for dummy bytes. The session precompile dispatch is
    // covered by the runtime integration tests via real //Alice..//Dave
    // session keys.
    const signers = await ethers.getSigners();
    const signer = signers[5];
    const stash = await signer.getAddress();
    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING, signer);

    const bondAmount = ethers.parseEther("2000");

    const payee = { kind: 0, account: ethers.ZeroAddress };
    await (await staking.bond(bondAmount, payee)).wait();
    const ledger = await getStakingLedger(stash);
    expect(ledger.active).to.equal(bondAmount);

    await (await staking.validate({ commissionPercent: 1, blocked: false })).wait();

    const startEra = Number(await staking.currentEra());
    await waitForEra(startEra + 1);

    await (await staking.unbond(bondAmount)).wait();
    await waitForEra(startEra + 3);

    await (await staking.withdrawUnbonded(0)).wait();

    const finalLedger = await getStakingLedger(stash);
    expect(finalLedger.active).to.equal(0n);
  });
});
