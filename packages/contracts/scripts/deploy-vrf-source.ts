import { ethers } from "hardhat";

async function main() {
  const vrfCoordinator = process.env.VRF_COORDINATOR_ADDRESS;
  const keyHash = process.env.VRF_KEY_HASH;
  const subscriptionId = process.env.VRF_SUBSCRIPTION_ID;

  if (!vrfCoordinator || !keyHash || !subscriptionId) {
    throw new Error(
      "Required env vars: VRF_COORDINATOR_ADDRESS, VRF_KEY_HASH, VRF_SUBSCRIPTION_ID"
    );
  }

  const factory = await ethers.getContractFactory("ChainlinkVRFResultSource");
  const contract = await factory.deploy(vrfCoordinator, keyHash, subscriptionId);
  await contract.waitForDeployment();

  const address = await contract.getAddress();
  console.log(`ChainlinkVRFResultSource deployed to: ${address}`);
  console.log(`Add ${address} as consumer in VRF subscription dashboard`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
