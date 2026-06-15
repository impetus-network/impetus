import {sts, Block, Bytes, Option, Result, EventType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const endowed =  {
    name: 'Balances.Endowed',
    /**
     * An account was created with some free balance.
     */
    v9: new EventType(
        'Balances.Endowed',
        sts.struct({
            account: v9.AccountId20,
            freeBalance: sts.bigint(),
        })
    ),
}

export const transfer =  {
    name: 'Balances.Transfer',
    /**
     * Transfer succeeded.
     */
    v9: new EventType(
        'Balances.Transfer',
        sts.struct({
            from: v9.AccountId20,
            to: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const balanceSet =  {
    name: 'Balances.BalanceSet',
    /**
     * A balance was set by root.
     */
    v9: new EventType(
        'Balances.BalanceSet',
        sts.struct({
            who: v9.AccountId20,
            free: sts.bigint(),
        })
    ),
}

export const reserved =  {
    name: 'Balances.Reserved',
    /**
     * Some balance was reserved (moved from free to reserved).
     */
    v9: new EventType(
        'Balances.Reserved',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const unreserved =  {
    name: 'Balances.Unreserved',
    /**
     * Some balance was unreserved (moved from reserved to free).
     */
    v9: new EventType(
        'Balances.Unreserved',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const deposit =  {
    name: 'Balances.Deposit',
    /**
     * Some amount was deposited (e.g. for transaction fees).
     */
    v9: new EventType(
        'Balances.Deposit',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const withdraw =  {
    name: 'Balances.Withdraw',
    /**
     * Some amount was withdrawn from the account (e.g. for transaction fees).
     */
    v9: new EventType(
        'Balances.Withdraw',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const slashed =  {
    name: 'Balances.Slashed',
    /**
     * Some amount was removed from the account (e.g. for misbehavior).
     */
    v9: new EventType(
        'Balances.Slashed',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const minted =  {
    name: 'Balances.Minted',
    /**
     * Some amount was minted into an account.
     */
    v9: new EventType(
        'Balances.Minted',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}

export const burned =  {
    name: 'Balances.Burned',
    /**
     * Some amount was burned from an account.
     */
    v9: new EventType(
        'Balances.Burned',
        sts.struct({
            who: v9.AccountId20,
            amount: sts.bigint(),
        })
    ),
}
