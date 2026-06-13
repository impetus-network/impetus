import {sts, Block, Bytes, Option, Result, EventType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const created =  {
    name: 'NominationPools.Created',
    /**
     * A pool has been created.
     */
    v9: new EventType(
        'NominationPools.Created',
        sts.struct({
            depositor: v9.AccountId20,
            poolId: sts.number(),
        })
    ),
}

export const bonded =  {
    name: 'NominationPools.Bonded',
    /**
     * A member has became bonded in a pool.
     */
    v9: new EventType(
        'NominationPools.Bonded',
        sts.struct({
            member: v9.AccountId20,
            poolId: sts.number(),
            bonded: sts.bigint(),
            joined: sts.boolean(),
        })
    ),
}

export const paidOut =  {
    name: 'NominationPools.PaidOut',
    /**
     * A payout has been made to a member.
     */
    v9: new EventType(
        'NominationPools.PaidOut',
        sts.struct({
            member: v9.AccountId20,
            poolId: sts.number(),
            payout: sts.bigint(),
        })
    ),
}

export const unbonded =  {
    name: 'NominationPools.Unbonded',
    /**
     * A member has unbonded from their pool.
     * 
     * - `balance` is the corresponding balance of the number of points that has been
     *   requested to be unbonded (the argument of the `unbond` transaction) from the bonded
     *   pool.
     * - `points` is the number of points that are issued as a result of `balance` being
     * dissolved into the corresponding unbonding pool.
     * - `era` is the era in which the balance will be unbonded.
     * In the absence of slashing, these values will match. In the presence of slashing, the
     * number of points that are issued in the unbonding pool will be less than the amount
     * requested to be unbonded.
     */
    v9: new EventType(
        'NominationPools.Unbonded',
        sts.struct({
            member: v9.AccountId20,
            poolId: sts.number(),
            balance: sts.bigint(),
            points: sts.bigint(),
            era: sts.number(),
        })
    ),
}

export const withdrawn =  {
    name: 'NominationPools.Withdrawn',
    /**
     * A member has withdrawn from their pool.
     * 
     * The given number of `points` have been dissolved in return of `balance`.
     * 
     * Similar to `Unbonded` event, in the absence of slashing, the ratio of point to balance
     * will be 1.
     */
    v9: new EventType(
        'NominationPools.Withdrawn',
        sts.struct({
            member: v9.AccountId20,
            poolId: sts.number(),
            balance: sts.bigint(),
            points: sts.bigint(),
        })
    ),
}

export const stateChanged =  {
    name: 'NominationPools.StateChanged',
    /**
     * The state of a pool has changed
     */
    v9: new EventType(
        'NominationPools.StateChanged',
        sts.struct({
            poolId: sts.number(),
            newState: v9.PoolState,
        })
    ),
}

export const memberRemoved =  {
    name: 'NominationPools.MemberRemoved',
    /**
     * A member has been removed from a pool.
     * 
     * The removal can be voluntary (withdrawn all unbonded funds) or involuntary (kicked).
     * Any funds that are still delegated (i.e. dangling delegation) are released and are
     * represented by `released_balance`.
     */
    v9: new EventType(
        'NominationPools.MemberRemoved',
        sts.struct({
            poolId: sts.number(),
            member: v9.AccountId20,
            releasedBalance: sts.bigint(),
        })
    ),
}

export const poolCommissionUpdated =  {
    name: 'NominationPools.PoolCommissionUpdated',
    /**
     * A pool's commission setting has been changed.
     */
    v9: new EventType(
        'NominationPools.PoolCommissionUpdated',
        sts.struct({
            poolId: sts.number(),
            current: sts.option(() => sts.tuple(() => [v9.Perbill, v9.AccountId20])),
        })
    ),
}
