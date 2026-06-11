import { z } from "zod";

const envSchema = z.object({
  DATABASE_URL: z.string().min(1),
  NEXT_PUBLIC_CHAIN_ID: z.coerce.number().default(322644),
  NEXT_PUBLIC_CHAIN_NAME: z.string().default("Impulse"),
  NEXT_PUBLIC_TOKEN_SYMBOL: z.string().default("IPL"),
  NEXT_PUBLIC_TOKEN_DECIMALS: z.coerce.number().default(18),
  NEXT_PUBLIC_EXPLORER_URL: z.string().optional(),
});

// Skip validation during build (no DB available); validate at runtime only
const isBuildPhase =
  process.env.NEXT_PHASE === "phase-production-build" ||
  process.env.NODE_ENV === undefined;

export const env = isBuildPhase
  ? (process.env as unknown as z.infer<typeof envSchema>)
  : envSchema.parse(process.env);
