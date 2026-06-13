import {sts, Block, Bytes, Option, Result, CallType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const nominate =  {
    name: 'Staking.nominate',
    /**
     * Declare the desire to nominate `targets` for the origin controller.
     * 
     * Effects will be felt at the beginning of the next era.
     * 
     * The dispatch origin for this call must be _Signed_ by the controller, not the stash.
     * 
     * ## Complexity
     * - The transaction's complexity is proportional to the size of `targets` (N)
     * which is capped at CompactAssignments::LIMIT (T::MaxNominations).
     * - Both the reads and writes follow a similar pattern.
     */
    v9: new CallType(
        'Staking.nominate',
        sts.struct({
            targets: sts.array(() => v9.AccountId20),
        })
    ),
}
