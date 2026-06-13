import {sts, Block, Bytes, Option, Result, EventType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

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
