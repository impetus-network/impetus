import { ethers } from "hardhat";

async function main() {
  const trustedRelayer = process.env.TRUSTED_RELAYER_ADDRESS;

  if (!trustedRelayer) {
    throw new Error("Required env var: TRUSTED_RELAYER_ADDRESS");
  }

  const verifierFactory = await ethers.getContractFactory("SingleRelayerVerifier");
  const verifier = await verifierFactory.deploy(trustedRelayer);
  await verifier.waitForDeployment();
  const verifierAddress = await verifier.getAddress();
  console.log(`SingleRelayerVerifier deployed to: ${verifierAddress}`);

  const receiverFactory = await ethers.getContractFactory("CrossChainResultReceiver");
  const receiver = await receiverFactory.deploy(verifierAddress);
  await receiver.waitForDeployment();
  const receiverAddress = await receiver.getAddress();
  console.log(`CrossChainResultReceiver deployed to: ${receiverAddress}`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
