import {sts, Block, Bytes, Option, Result, StorageType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const account =  {
    /**
     *  The full account information for a particular account ID.
     */
    v9: new StorageType('System.Account', 'Default', [v9.AccountId20], v9.AccountInfo) as AccountV9,
}

/**
 *  The full account information for a particular account ID.
 */
export interface AccountV9  {
    is(block: RuntimeCtx): boolean
    getDefault(block: Block): v9.AccountInfo
    get(block: Block, key: v9.AccountId20): Promise<(v9.AccountInfo | undefined)>
    getMany(block: Block, keys: v9.AccountId20[]): Promise<(v9.AccountInfo | undefined)[]>
    getKeys(block: Block): Promise<v9.AccountId20[]>
    getKeys(block: Block, key: v9.AccountId20): Promise<v9.AccountId20[]>
    getKeysPaged(pageSize: number, block: Block): AsyncIterable<v9.AccountId20[]>
    getKeysPaged(pageSize: number, block: Block, key: v9.AccountId20): AsyncIterable<v9.AccountId20[]>
    getPairs(block: Block): Promise<[k: v9.AccountId20, v: (v9.AccountInfo | undefined)][]>
    getPairs(block: Block, key: v9.AccountId20): Promise<[k: v9.AccountId20, v: (v9.AccountInfo | undefined)][]>
    getPairsPaged(pageSize: number, block: Block): AsyncIterable<[k: v9.AccountId20, v: (v9.AccountInfo | undefined)][]>
    getPairsPaged(pageSize: number, block: Block, key: v9.AccountId20): AsyncIterable<[k: v9.AccountId20, v: (v9.AccountInfo | undefined)][]>
}
