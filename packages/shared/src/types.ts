import type { Address, Hex } from "viem";

export interface GaslessRule {
  id: string;
  contract: Address;
  selector: Hex;
  enabled: boolean;
  minValue: bigint;
  updatedAtBlock: bigint;
}
