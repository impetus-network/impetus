import { expect } from "chai";
import { ethers } from "hardhat";
import { ADDRESSES } from "./helpers/staking-helpers";

describe("DELEGATECALL guard across NPoS precompiles", function () {
  this.timeout(2 * 60 * 1000);

  // Each entry uses a representative WRITE selector. Views do not call
  // delegate_guard (they are pure storage reads), so only write entries
  // are guaranteed to reject delegatecall by design.
  const writeTargets = [
    { name: "Staking.chill", iface: "IStaking", addr: ADDRESSES.STAKING, fn: "chill", args: [] as unknown[] },
    { name: "Session.purgeKeys", iface: "ISession", addr: ADDRESSES.SESSION, fn: "purgeKeys", args: [] as unknown[] },
    { name: "Pools.claimPayout", iface: "INominationPools", addr: ADDRESSES.POOLS, fn: "claimPayout", args: [] as unknown[] },
    { name: "FastUnstake.deregister", iface: "IFastUnstake", addr: ADDRESSES.FAST_UNSTAKE, fn: "deregister", args: [] as unknown[] },
    { name: "Treasury.payout", iface: "ITreasury", addr: ADDRESSES.TREASURY, fn: "payout", args: [0] as unknown[] },
    { name: "StakingAdmin.forceNewEra", iface: "IStakingAdmin", addr: ADDRESSES.STAKING_ADMIN, fn: "forceNewEra", args: [] as unknown[] },
  ];

  it("write entries reject delegatecall with the canonical revert reason", async () => {
    // `proxy.delegate(...)` returns `(bool, bytes)` from the inner
    // delegatecall, so the outer tx succeeds even when the precompile's
    // delegate_guard reverts. We use a staticCall to read both halves of
    // the return tuple and decode the revert reason from the bytes payload
    // (selector 0x08c379a0 = Error(string)).
    const proxyFactory = await ethers.getContractFactory("DelegateProxy");
    const proxy = await proxyFactory.deploy();
    await proxy.waitForDeployment();

    const staking = await ethers.getContractAt("IStaking", ADDRESSES.STAKING);
    const data = staking.interface.encodeFunctionData("chill", []);
    const [ok, ret] = await proxy.delegate.staticCall(ADDRESSES.STAKING, data);
    expect(ok).to.equal(false);

    // Error(string) selector + 32-byte offset + 32-byte length + utf8 body.
    expect(ret.slice(0, 10)).to.equal("0x08c379a0");
    const reason = new ethers.AbiCoder().decode(["string"], "0x" + ret.slice(10))[0];
    expect(reason).to.include("DELEGATECALL");
  });

  it("BagsList.rebag rejects delegatecall (self-target)", async () => {
    const proxyFactory = await ethers.getContractFactory("DelegateProxy");
    const proxy = await proxyFactory.deploy();
    await proxy.waitForDeployment();

    const proxyAddr = await proxy.getAddress();
    const bags = await ethers.getContractAt("IBagsList", ADDRESSES.BAGS_LIST);
    const data = bags.interface.encodeFunctionData("rebag", [proxyAddr]);
    const [ok] = await proxy.delegate.staticCall(ADDRESSES.BAGS_LIST, data);
    expect(ok, "BagsList.rebag via delegatecall should fail").to.equal(false);
  });

  it("every write entry across NPoS precompiles fails via delegatecall", async () => {
    const proxyFactory = await ethers.getContractFactory("DelegateProxy");
    const proxy = await proxyFactory.deploy();
    await proxy.waitForDeployment();

    for (const t of writeTargets) {
      const c = await ethers.getContractAt(t.iface, t.addr);
      const data = c.interface.encodeFunctionData(t.fn, t.args);
      const [ok] = await proxy.delegate.staticCall(t.addr, data);
      expect(ok, `${t.name} via delegatecall should fail`).to.equal(false);
    }
  });
});
