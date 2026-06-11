"use client";

import { artemis } from "~/config/chains";

export function useTargetNetwork() {
  return { targetNetwork: artemis };
}
