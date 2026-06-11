import { expect } from "chai";
import { ethers } from "hardhat";

describe("CrossChainResultReceiver", function () {
  const RELAYER_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

  async function deployReceiver() {
    const [owner, nonOwner] = await ethers.getSigners();
    const relayer = new ethers.Wallet(RELAYER_KEY);

    const verifierFactory = await ethers.getContractFactory("SingleRelayerVerifier");
    const verifier = await verifierFactory.deploy(relayer.address);
    await verifier.waitForDeployment();

    const receiverFactory = await ethers.getContractFactory("CrossChainResultReceiver");
    const receiver = await receiverFactory.deploy(await verifier.getAddress());
    await receiver.waitForDeployment();

    return { receiver, verifier, relayer, owner, nonOwner };
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

  const ROUND_ID = 1n;
  const SPECIAL_PRIZE = 42;
  const ALL_PRIZES = Array.from({ length: 27 }, (_, i) => i % 100);

  describe("requestResult", function () {
    it("emits ResultRequested and marks round as requested", async function () {
      const { receiver } = await deployReceiver();
      await expect(receiver.requestResult(ROUND_ID))
        .to.emit(receiver, "ResultRequested")
        .withArgs(ROUND_ID);
      expect(await receiver.requested(ROUND_ID)).to.be.true;
    });

    it("reverts if round already requested", async function () {
      const { receiver } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);
      await expect(receiver.requestResult(ROUND_ID))
        .to.be.revertedWithCustomError(receiver, "RoundAlreadyRequested");
    });
  });

  describe("submitResult", function () {
    it("stores result with valid signature for requested round", async function () {
      const { receiver, relayer } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);
      const signature = await signResult(relayer, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);
      await expect(
        receiver.submitResult(ROUND_ID, { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES }, signature)
      ).to.emit(receiver, "ResultFulfilled").withArgs(ROUND_ID);
      expect(await receiver.hasResult(ROUND_ID)).to.be.true;
      const result = await receiver.getResult(ROUND_ID);
      expect(result.specialPrize).to.equal(SPECIAL_PRIZE);
      expect(result.allPrizes).to.have.lengthOf(27);
      expect(result.allPrizes[0]).to.equal(ALL_PRIZES[0]);
    });

    it("reverts if round not requested", async function () {
      const { receiver, relayer } = await deployReceiver();
      const signature = await signResult(relayer, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);
      await expect(
        receiver.submitResult(ROUND_ID, { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES }, signature)
      ).to.be.revertedWithCustomError(receiver, "RoundNotRequested");
    });

    it("reverts if round already fulfilled", async function () {
      const { receiver, relayer } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);
      const signature = await signResult(relayer, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);
      await receiver.submitResult(ROUND_ID, { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES }, signature);
      await expect(
        receiver.submitResult(ROUND_ID, { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES }, signature)
      ).to.be.revertedWithCustomError(receiver, "RoundAlreadyFulfilled");
    });

    it("reverts with invalid signature", async function () {
      const { receiver } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);
      const wrongSigner = new ethers.Wallet("0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a");
      const signature = await signResult(wrongSigner, ROUND_ID, SPECIAL_PRIZE, ALL_PRIZES);
      await expect(
        receiver.submitResult(ROUND_ID, { specialPrize: SPECIAL_PRIZE, allPrizes: ALL_PRIZES }, signature)
      ).to.be.revertedWithCustomError(receiver, "InvalidProof");
    });
  });

  describe("getResult", function () {
    it("reverts if round not fulfilled", async function () {
      const { receiver } = await deployReceiver();
      await receiver.requestResult(ROUND_ID);
      await expect(receiver.getResult(ROUND_ID))
        .to.be.revertedWithCustomError(receiver, "RoundNotFulfilled");
    });
  });

  describe("setVerifier", function () {
    it("allows owner to change verifier", async function () {
      const { receiver, owner } = await deployReceiver();
      const newVerifierFactory = await ethers.getContractFactory("SingleRelayerVerifier");
      const newVerifier = await newVerifierFactory.deploy(owner.address);
      await newVerifier.waitForDeployment();
      await receiver.setVerifier(await newVerifier.getAddress());
      expect(await receiver.verifier()).to.equal(await newVerifier.getAddress());
    });

    it("reverts if called by non-owner", async function () {
      const { receiver, nonOwner } = await deployReceiver();
      await expect(
        receiver.connect(nonOwner).setVerifier(ethers.ZeroAddress)
      ).to.be.revertedWithCustomError(receiver, "OwnableUnauthorizedAccount");
    });
  });
});
