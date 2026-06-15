import {sts, Block, Bytes, Option, Result, StorageType, RuntimeCtx} from '../support'

export const totalIssuance =  {
    /**
     *  The total units issued in the system.
     */
    v9: new StorageType('Balances.TotalIssuance', 'Default', [], sts.bigint()) as TotalIssuanceV9,
}

/**
 *  The total units issued in the system.
 */
export interface TotalIssuanceV9  {
    is(block: RuntimeCtx): boolean
    getDefault(block: Block): bigint
    get(block: Block): Promise<(bigint | undefined)>
}
