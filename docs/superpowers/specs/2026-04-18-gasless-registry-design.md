# Gasless Registry Design Spec

## Overview

Add a chain-level gasless registry for selected EVM contract functions. The
registry lets an admin/root-compatible origin whitelist
`contract address + function selector` pairs so normal EVM transactions can
execute without charging the caller's native balance for EVM gas.

This MVP assumes the admin only approves functions that already have bounded
resource usage or economic friction, such as native value transfer, contract
enforced token transfer, burn, stake, escrow, deposit, one-time claim, or other
reviewed bounded state transitions.

## Goals

- Support gasless EVM calls for admin-approved contract function selectors.
- Keep the registry generic and independent of the betting pallet.
- Use admin/root-compatible management in MVP while leaving room for
  governance-managed rules later.
- Keep the hot path small: one registry lookup for EVM call eligibility.
- Add optional native `msg.value` minimums for rules where the registry can
  enforce economic friction directly.
- Add a runtime-wide gas cap for gasless eligibility.
- Fall back to paid execution when a rule is missing, disabled, underfunded, or
  above the gas cap.

## Non-Goals

- No relayer or meta-transaction service.
- No paymaster or ERC-4337-style account abstraction.
- No sponsor budget accounting.
- No per-account quota, cooldown, or usage tracking in MVP.
- No self-service opt-in for arbitrary contract owners.
- No ABI registry beyond the 4-byte function selector.
- No general free execution for every call to a contract.
- No zero-balance gasless UX in MVP. Callers must still pass Frontier
  transaction-pool validation. The gasless rule prevents fee withdrawal during
  execution; it does not bypass all pool admission checks.
- No use of `Pays::No` for EVM transactions. `Pays::No` applies to FRAME
  dispatchables and is not the mechanism for Frontier self-contained Ethereum
  transactions.

## Architecture

Add a new runtime pallet named `pallet-gasless-registry`.

The pallet owns gasless policy. It does not execute EVM calls, does not call
application contracts, and does not parse ABI beyond the selector. Runtime EVM
fee handling queries the pallet before deciding whether to charge EVM gas fees
from the caller's native balance.

The policy key is:

```text
contract: H160
selector: [u8; 4]
```

The pallet exposes helper logic equivalent to:

```text
evaluate(contract, calldata, value, gas_limit) -> GaslessDecision
```

`GaslessDecision` is either:

```text
Gasless
Paid
```

## Storage

```text
Rules[(H160, [u8; 4])] = Rule {
  enabled: bool,
  min_value: U256,
}
```

`min_value` is the minimum native EVM transaction value required for gasless
eligibility. Set `min_value = 0` only when the admin has reviewed the target
function and decided it is safe without native value friction.

The runtime also defines:

```text
MaxGaslessGasLimit: u64
```

`MaxGaslessGasLimit` is a chain-wide maximum EVM transaction gas limit for
gasless eligibility. Calls above this cap fall back to paid execution. The cap
is deliberately not per-rule in MVP to keep admin operations simple while still
preventing a whitelisted selector from requesting the full block gas limit
under subsidy.

## Calls

### set_rule

```text
set_rule(contract, selector, min_value, enabled)
```

Allowed only by the configured management origin.

There is no input validation beyond origin checking. `min_value = 0` is valid
for admin-reviewed bounded functions, and `enabled = false` is valid for
staging or temporarily disabling a rule while preserving its configuration.

Effects:

```text
Rules[(contract, selector)] = Rule { enabled, min_value }
emit RuleSet
```

### remove_rule

```text
remove_rule(contract, selector)
```

Allowed only by the configured management origin.

Effects:

```text
remove Rules[(contract, selector)]
emit RuleRemoved
```

Removing a missing rule returns `RuleNotFound`.

## Data Flow

### Admin Flow

```text
Admin/root calls set_rule(contract, selector, min_value, enabled)
  store the rule
  emit RuleSet
```

### User Flow

```text
User sends EVM transaction
  runtime extracts target contract, calldata, tx value, and gas limit
  if transaction is contract creation -> paid
  if calldata has fewer than 4 bytes -> paid
  selector = calldata[0..4]
  lookup Rules[(contract, selector)]
  missing rule -> paid
  disabled rule -> paid
  tx.value < rule.min_value -> paid
  gas_limit > MaxGaslessGasLimit -> paid
  otherwise -> gasless
```

Gasless eligibility must not make the target EVM call succeed or fail
differently. It only changes whether the native balance is charged for EVM gas.

## Runtime Integration

This repo uses Frontier self-contained Ethereum transactions:

```text
RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction })
```

Those transactions are converted as bare extrinsics and dispatched through
`fp_self_contained`, so the normal Substrate signed-extra fee path is not the
right primary hook for gasless EVM calls. `pallet_transaction_payment` still
handles regular signed Substrate extrinsics, but EVM gas charging happens in
the Frontier EVM execution path through `pallet_evm::Config::OnChargeTransaction`.

The intended integration point is the Frontier EVM execution path, with a
custom EVM fee charger plus a transaction-context provider, or an equivalent
wrapper around `pallet_evm::OnChargeEVMTransaction`. In the current runtime,
`pallet_evm::Config::OnChargeTransaction` is configured as `()`, which uses
Frontier's default EVM fee adapter. The implementation should replace that with
an explicit custom type that delegates to the default paid behavior unless the
registry marks the call gasless.

Frontier's `OnChargeEVMTransaction` trait is not enough by itself to evaluate
gasless eligibility. Its fee hooks receive only the caller and fee amounts:

```text
withdraw_fee(who, fee)
correct_and_deposit_fee(who, corrected_fee, base_fee, already_withdrawn)
pay_priority_fee(tip)
```

They do not receive the target contract, calldata, transaction value, or gas
limit. The implementation must therefore provide that transaction context from
a higher layer before the fee hook decides whether to waive withdrawal.
Acceptable approaches include a scoped runtime context set around the EVM runner
call, a narrow wrapper at the `pallet_ethereum::transact` or self-contained call
boundary, or another explicit mechanism that is transactional, cleared after the
call, and unavailable to unrelated extrinsics. The design must not depend on
`withdraw_fee` being able to reconstruct the full Ethereum transaction from its
current trait arguments.

The implementation plan must choose the narrowest hook that can:

- Identify Ethereum/EVM call transactions.
- Extract target contract, calldata selector, transaction value, and gas limit.
- Waive EVM gas fee withdrawal only when the registry returns `Gasless`.
- Preserve normal paid execution for all other transactions.
- Avoid mutating registry state during pool validation, fee estimation, or
  dry-run execution.

For MVP, transaction-pool validation keeps Frontier's default balance checks.
In this runtime, `pallet_ethereum::validate_transaction_in_pool` validates EVM
transactions with `with_balance_for(&who)`. Supporting callers with no native
balance would require modifying the self-contained validation path to recognize
gasless-eligible calls before `with_balance_for`, and to do so read-only. That
is future work, not part of this MVP.

Fallback paid execution means the normal Frontier EVM fee path must remain
available. If a rule is missing, disabled, under `min_value`, or above
`MaxGaslessGasLimit`, the transaction must pass through the normal
`OnChargeEVMTransaction` withdrawal and refund flow.

RPC `eth_call` and `eth_estimateGas` use the runtime API path that invokes the
EVM runner with `is_transactional = false`; in that mode the computed EVM fee is
zero. A custom fee charger may still be called with a zero fee, but it must not
perform any stateful registry mutation.

## Admin Approval Policy

The registry intentionally relies on admin review instead of generic quotas in
MVP. Admin/root should approve only selectors that satisfy at least one of:

- Native `msg.value` transfer, enforced with `min_value`.
- Contract-enforced token transfer, burn, stake, escrow, or deposit.
- One-time or naturally bounded state transition.
- A reviewed low-risk operation with bounded gas and bounded state growth.

If a selector can be called repeatedly for free without economic friction or a
contract-level bound, it should not be registered in MVP.

## Performance And Weight Accounting

Every EVM transaction will pass through the custom EVM fee charger. The MVP
accepts one registry storage lookup on this hot path to determine whether the
target `(contract, selector)` is registered. This overhead is acceptable for the
current chain scale, but it must be benchmarked and included in block-weight
accounting.

If profiling shows this lookup is material, later versions can add
contract-level indexes or other early-exit structures. Those structures still
require storage access and should be justified by measured benefit.

Gasless evaluation adds a rule read. That cost is paid by the chain as part of
the gasless subsidy, not by the caller as an EVM fee. To avoid creating
unmetered block work, the implementation must account for registry reads in
runtime weight. Acceptable approaches include:

- adding a benchmarked worst-case registry overhead to Ethereum transaction
  weight calculation when the registry pallet is enabled;
- translating benchmarked registry overhead into an EVM gas reserve and
  requiring gasless-eligible calls to have enough gas limit to cover it;
- or another explicit mechanism that makes registry storage work visible to
  block-weight limits.

The implementation must not rely only on EVM gas metering, because registry
storage reads happen outside the EVM contract execution meter.

Gasless execution means the chain forgoes collecting the EVM base fee and
priority fee for that eligible call. No treasury account pays another account
in MVP. The cost is controlled by admin approval, `min_value`,
`MaxGaslessGasLimit`, and block gas/weight limits.

## Error Handling

Admin calls can fail:

```text
remove_rule missing rule -> RuleNotFound
unauthorized management call -> BadOrigin
```

User EVM calls should fall back to paid execution:

```text
not an EVM call -> paid
contract creation -> paid
missing calldata selector -> paid
unknown rule -> paid
disabled rule -> paid
tx.value below min_value -> paid
gas_limit above MaxGaslessGasLimit -> paid
```

The registry must not make a user transaction fail just because gasless
eligibility is unavailable.

## Edge Cases

- Contract creation transactions are paid.
- Calls with calldata shorter than 4 bytes are paid.
- Calls to precompile addresses are paid unless explicitly registered.
- Four-byte selector collisions are scoped by contract address. This is an
  acceptable MVP risk because only admin/root-registered `(contract, selector)`
  pairs are eligible.
- Registry checks do not change EVM reentrancy behavior. Contract reentrancy
  protections remain the responsibility of the target contract.
- Rule changes apply immediately.
- Because MVP has no usage storage, there is no usage cleanup path and no
  usage-related state growth.
- Rule struct changes after deployment require storage-versioned migrations.
  Adding a new storage item does not require a migration, but changing the
  encoding or interpretation of existing `Rule` values does.

## Events

```text
RuleSet {
  contract: H160,
  selector: [u8; 4],
  enabled: bool,
  min_value: U256,
}

RuleRemoved {
  contract: H160,
  selector: [u8; 4],
}
```

## Weights And Benchmarking

All new dispatchables must have benchmark-backed weights before production use.
The MVP should include benchmarks for:

- `set_rule`
- `remove_rule`
- gasless evaluation in the EVM path

The gasless evaluation helper must avoid unbounded work and should account for
the storage reads it performs when used from the EVM execution path. Hardcoded
placeholder weights are acceptable only during an early spike and must be
replaced before the feature is treated as complete.

Because gasless evaluation is a helper invoked from the EVM fee path rather
than a public dispatchable, its benchmarking may need a dedicated benchmark
that exercises the helper, or a benchmarked wrapper used only for measuring the
storage read cost. It should not be treated as covered merely because
`set_rule` and `remove_rule` are benchmarked.

## Testing

### Pallet Unit Tests

- `set_rule` stores an enabled rule.
- `set_rule` stores `min_value`.
- `remove_rule` deletes an existing rule.
- `remove_rule` rejects missing rules.
- A matching call above `MaxGaslessGasLimit` returns paid.
- A matching call below `min_value` returns paid.
- A matching enabled rule at or above `min_value` and under the gas cap returns
  gasless.
- Missing, disabled, creation, and short-calldata cases return paid.

### Runtime / Frontier Integration Tests

- A registered EVM call is fee-free when it matches the rule.
- Paid fallback still executes the contract function.
- Non-registered EVM calls remain paid.
- Contract creation remains paid.
- Gasless-eligible EVM calls still need to satisfy MVP transaction-pool balance
  validation.
- Dry-run or estimation does not mutate registry state.
- `eth_call` and `eth_estimateGas` do not mutate registry state even if the
  custom EVM fee charger is invoked with zero fee.
- Registry overhead is included in runtime block-weight accounting.
- A registered call above `MaxGaslessGasLimit` falls back to paid execution.
- A registered call below `min_value` falls back to paid execution.

### TypeScript / Hardhat Tests

- Deploy a minimal test contract.
- Register one function selector.
- Call the registered selector with enough native value and under the gas cap.
- Verify the caller balance does not decrease by EVM gas fee, excluding the
  explicit value transfer.
- Call the registered selector below `min_value`.
- Verify the function still executes and the caller pays the normal fee.

The test contract should be independent of the betting precompile so the
registry remains generic.

## Open Implementation Risk

The largest risk is the exact Frontier fee integration point. The behavior is
clear, but the implementation must verify where this runtime can safely waive
EVM gas fee withdrawal based on calldata, value, and gas limit while keeping
transaction-pool validation read-only and preserving paid fallback behavior.
This risk belongs in the implementation plan and should be resolved before
writing the runtime integration code.

Zero-balance gasless support is intentionally deferred. If that requirement is
added later, the design must extend Frontier self-contained transaction-pool
validation, not only EVM fee withdrawal.
