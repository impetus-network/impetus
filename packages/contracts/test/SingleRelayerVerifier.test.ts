import { expect } from "chai";
import { ethers } from "hardhat";

describe("SingleRelayerVerifier", function () {
  const TRUSTED_RELAYER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
  const OTHER_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

  async function deployVerifier() {
    const trustedRelayer = new ethers.Wallet(TRUSTED_RELAYER_KEY);
    const otherSigner = new ethers.Wallet(OTHER_KEY);

    const factory = await ethers.getContractFactory("SingleRelayerVerifier");
    const verifier = await factory.deploy(trustedRelayer.address);
    await verifier.waitForDeployment();

    return { verifier, trustedRelayer, otherSigner };
  }

  async function signResult(
    signer: InstanceType<typeof ethers.Wallet>,
    roundId: bigint,
    specialPrize: number,
    allPrizes: number[]
  ): Promise<string> {
    const hash = ethers.keccak256(
      ethers.AbiCoder.defaultAbiCoder().encode(
        ["uint256", "uint8", "uint8[]"],
        [roundId, specialPrize, allPrizes]
      )
    );
    return signer.signMessage(ethers.getBytes(hash));
  }

  it("returns true for valid signature from trusted relayer", async function () {
    const { verifier, trustedRelayer } = await deployVerifier();
    const roundId = 1n;
    const specialPrize = 42;
    const allPrizes = Array.from({ length: 27 }, (_, i) => i % 100);

    const signature = await signResult(trustedRelayer, roundId, specialPrize, allPrizes);

    const result = await verifier.verify(
      roundId,
      { specialPrize, allPrizes },
      signature
    );
    expect(result).to.be.true;
  });

  it("returns false for signature from wrong signer", async function () {
    const { verifier, otherSigner } = await deployVerifier();
    const roundId = 1n;
    const specialPrize = 42;
    const allPrizes = Array.from({ length: 27 }, (_, i) => i % 100);

    const signature = await signResult(otherSigner, roundId, specialPrize, allPrizes);

    const result = await verifier.verify(
      roundId,
      { specialPrize, allPrizes },
      signature
    );
    expect(result).to.be.false;
  });

  it("returns false for tampered result", async function () {
    const { verifier, trustedRelayer } = await deployVerifier();
    const roundId = 1n;
    const specialPrize = 42;
    const allPrizes = Array.from({ length: 27 }, (_, i) => i % 100);

    const signature = await signResult(trustedRelayer, roundId, specialPrize, allPrizes);

    const result = await verifier.verify(
      roundId,
      { specialPrize: 99, allPrizes },
      signature
    );
    expect(result).to.be.false;
  });

  it("exposes trustedRelayer address", async function () {
    const { verifier, trustedRelayer } = await deployVerifier();
    expect(await verifier.trustedRelayer()).to.equal(trustedRelayer.address);
  });
});
