import { onchainTable } from "ponder";

export const gaslessRules = onchainTable("gasless_rules", (t) => ({
  id: t.text().primaryKey(),
  contract: t.hex().notNull(),
  selector: t.hex().notNull(),
  enabled: t.boolean().notNull(),
  minValue: t.bigint().notNull(),
  updatedAtBlock: t.bigint().notNull(),
}));
