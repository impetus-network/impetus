import { ponder } from "ponder:registry";
import { gaslessRules } from "ponder:schema";

ponder.on("GaslessRegistry:RuleSet", async ({ event, context }) => {
  const id = `${event.args.contract_}-${event.args.selector}`;
  await context.db
    .insert(gaslessRules)
    .values({
      id,
      contract: event.args.contract_,
      selector: event.args.selector,
      enabled: event.args.enabled,
      minValue: event.args.minValue,
      updatedAtBlock: event.block.number,
    })
    .onConflictDoUpdate({
      enabled: event.args.enabled,
      minValue: event.args.minValue,
      updatedAtBlock: event.block.number,
    });
});

ponder.on("GaslessRegistry:RuleRemoved", async ({ event, context }) => {
  const id = `${event.args.contract_}-${event.args.selector}`;
  await context.db
    .insert(gaslessRules)
    .values({
      id,
      contract: event.args.contract_,
      selector: event.args.selector,
      enabled: false,
      minValue: 0n,
      updatedAtBlock: event.block.number,
    })
    .onConflictDoUpdate({
      enabled: false,
      minValue: 0n,
      updatedAtBlock: event.block.number,
    });
});
