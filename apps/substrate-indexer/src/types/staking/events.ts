import {sts, Block, Bytes, Option, Result, EventType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const eraPaid =  {
    name: 'Staking.EraPaid',
    /**
     * The era payout has been set; the first balance is the validator-payout; the second is
     * the remainder from the maximum amount of reward.
     */
    v9: new EventType(
        'Staking.EraPaid',
        sts.struct({
            eraIndex: sts.number(),
            validatorPayout: sts.bigint(),
            remainder: sts.bigint(),
        })
    ),
}

export const rewarded =  {
    name: 'Staking.Rewarded',
    /**
     * The nominator has been rewarded by this amount to this destination.
     */
    v9: new EventType(
        'Staking.Rewarded',
        sts.struct({
            stash: v9.AccountId20,
            dest: v9.RewardDestination,
            amount: sts.bigint(),
        })
    ),
}

export const slashed =  {
    name: 'Staking.Slashed',
    /**
     * A staker (validator or nominator) has been slashed by the given amount.
     */
    v9: new EventType(
        'Staking.Slashed',
        sts.struct({
            staker: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const stakersElected =  {
    name: 'Staking.StakersElected',
    /**
     * A new set of stakers was elected.
     */
    v9: new EventType(
        'Staking.StakersElected',
        sts.unit()
    ),
}

export const bonded =  {
    name: 'Staking.Bonded',
    /**
     * An account has bonded this amount. \[stash, amount\]
     * 
     * NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
     * it will not be emitted for staking rewards when they are added to stake.
     */
    v9: new EventType(
        'Staking.Bonded',
        sts.struct({
            stash: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const unbonded =  {
    name: 'Staking.Unbonded',
    /**
     * An account has unbonded this amount.
     */
    v9: new EventType(
        'Staking.Unbonded',
        sts.struct({
            stash: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const withdrawn =  {
    name: 'Staking.Withdrawn',
    /**
     * An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
     * from the unlocking queue.
     */
    v9: new EventType(
        'Staking.Withdrawn',
        sts.struct({
            stash: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const kicked =  {
    name: 'Staking.Kicked',
    /**
     * A nominator has been kicked from a validator.
     */
    v9: new EventType(
        'Staking.Kicked',
        sts.struct({
            nominator: v9.AccountId20,
            stash: v9.AccountId20,
        })
    ),
}

export const chilled =  {
    name: 'Staking.Chilled',
    /**
     * An account has stopped participating as either a validator or nominator.
     */
    v9: new EventType(
        'Staking.Chilled',
        sts.struct({
            stash: v9.AccountId20,
        })
    ),
}

export const payoutStarted =  {
    name: 'Staking.PayoutStarted',
    /**
     * A Page of stakers rewards are getting paid. `next` is `None` if all pages are claimed.
     */
    v9: new EventType(
        'Staking.PayoutStarted',
        sts.struct({
            eraIndex: sts.number(),
            validatorStash: v9.AccountId20,
            page: sts.number(),
            next: sts.option(() => sts.number()),
        })
    ),
}

export const validatorPrefsSet =  {
    name: 'Staking.ValidatorPrefsSet',
    /**
     * A validator has set their preferences.
     */
    v9: new EventType(
        'Staking.ValidatorPrefsSet',
        sts.struct({
            stash: v9.AccountId20,
            prefs: v9.ValidatorPrefs,
        })
    ),
}
