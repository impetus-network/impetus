import {sts, Block, Bytes, Option, Result, StorageType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const validators =  {
    /**
     *  The current set of validators.
     */
    v9: new StorageType('Session.Validators', 'Default', [], sts.array(() => v9.AccountId20)) as ValidatorsV9,
}

/**
 *  The current set of validators.
 */
export interface ValidatorsV9  {
    is(block: RuntimeCtx): boolean
    getDefault(block: Block): v9.AccountId20[]
    get(block: Block): Promise<(v9.AccountId20[] | undefined)>
}
