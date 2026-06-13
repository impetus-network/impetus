import {sts, Block, Bytes, Option, Result, StorageType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const ledger =  {
    /**
     *  Map from all (unlocked) "controller" accounts to the info regarding the staking.
     * 
     *  Note: All the reads and mutations to this storage *MUST* be done through the methods exposed
     *  by [`StakingLedger`] to ensure data and lock consistency.
     */
    v9: new StorageType('Staking.Ledger', 'Optional', [v9.AccountId20], v9.StakingLedger) as LedgerV9,
}

/**
 *  Map from all (unlocked) "controller" accounts to the info regarding the staking.
 * 
 *  Note: All the reads and mutations to this storage *MUST* be done through the methods exposed
 *  by [`StakingLedger`] to ensure data and lock consistency.
 */
export interface LedgerV9  {
    is(block: RuntimeCtx): boolean
    get(block: Block, key: v9.AccountId20): Promise<(v9.StakingLedger | undefined)>
    getMany(block: Block, keys: v9.AccountId20[]): Promise<(v9.StakingLedger | undefined)[]>
    getKeys(block: Block): Promise<v9.AccountId20[]>
    getKeys(block: Block, key: v9.AccountId20): Promise<v9.AccountId20[]>
    getKeysPaged(pageSize: number, block: Block): AsyncIterable<v9.AccountId20[]>
    getKeysPaged(pageSize: number, block: Block, key: v9.AccountId20): AsyncIterable<v9.AccountId20[]>
    getPairs(block: Block): Promise<[k: v9.AccountId20, v: (v9.StakingLedger | undefined)][]>
    getPairs(block: Block, key: v9.AccountId20): Promise<[k: v9.AccountId20, v: (v9.StakingLedger | undefined)][]>
    getPairsPaged(pageSize: number, block: Block): AsyncIterable<[k: v9.AccountId20, v: (v9.StakingLedger | undefined)][]>
    getPairsPaged(pageSize: number, block: Block, key: v9.AccountId20): AsyncIterable<[k: v9.AccountId20, v: (v9.StakingLedger | undefined)][]>
}

export const validators =  {
    /**
     *  The map from (wannabe) validator stash key to the preferences of that validator.
     * 
     *  TWOX-NOTE: SAFE since `AccountId` is a secure hash.
     */
    v9: new StorageType('Staking.Validators', 'Default', [v9.AccountId20], v9.ValidatorPrefs) as ValidatorsV9,
}

/**
 *  The map from (wannabe) validator stash key to the preferences of that validator.
 * 
 *  TWOX-NOTE: SAFE since `AccountId` is a secure hash.
 */
export interface ValidatorsV9  {
    is(block: RuntimeCtx): boolean
    getDefault(block: Block): v9.ValidatorPrefs
    get(block: Block, key: v9.AccountId20): Promise<(v9.ValidatorPrefs | undefined)>
    getMany(block: Block, keys: v9.AccountId20[]): Promise<(v9.ValidatorPrefs | undefined)[]>
    getKeys(block: Block): Promise<v9.AccountId20[]>
    getKeys(block: Block, key: v9.AccountId20): Promise<v9.AccountId20[]>
    getKeysPaged(pageSize: number, block: Block): AsyncIterable<v9.AccountId20[]>
    getKeysPaged(pageSize: number, block: Block, key: v9.AccountId20): AsyncIterable<v9.AccountId20[]>
    getPairs(block: Block): Promise<[k: v9.AccountId20, v: (v9.ValidatorPrefs | undefined)][]>
    getPairs(block: Block, key: v9.AccountId20): Promise<[k: v9.AccountId20, v: (v9.ValidatorPrefs | undefined)][]>
    getPairsPaged(pageSize: number, block: Block): AsyncIterable<[k: v9.AccountId20, v: (v9.ValidatorPrefs | undefined)][]>
    getPairsPaged(pageSize: number, block: Block, key: v9.AccountId20): AsyncIterable<[k: v9.AccountId20, v: (v9.ValidatorPrefs | undefined)][]>
}

export const erasValidatorReward =  {
    /**
     *  The total validator era payout for the last [`Config::HistoryDepth`] eras.
     * 
     *  Eras that haven't finished yet or has been removed doesn't have reward.
     */
    v9: new StorageType('Staking.ErasValidatorReward', 'Optional', [sts.number()], sts.bigint()) as ErasValidatorRewardV9,
}

/**
 *  The total validator era payout for the last [`Config::HistoryDepth`] eras.
 * 
 *  Eras that haven't finished yet or has been removed doesn't have reward.
 */
export interface ErasValidatorRewardV9  {
    is(block: RuntimeCtx): boolean
    get(block: Block, key: number): Promise<(bigint | undefined)>
    getMany(block: Block, keys: number[]): Promise<(bigint | undefined)[]>
    getKeys(block: Block): Promise<number[]>
    getKeys(block: Block, key: number): Promise<number[]>
    getKeysPaged(pageSize: number, block: Block): AsyncIterable<number[]>
    getKeysPaged(pageSize: number, block: Block, key: number): AsyncIterable<number[]>
    getPairs(block: Block): Promise<[k: number, v: (bigint | undefined)][]>
    getPairs(block: Block, key: number): Promise<[k: number, v: (bigint | undefined)][]>
    getPairsPaged(pageSize: number, block: Block): AsyncIterable<[k: number, v: (bigint | undefined)][]>
    getPairsPaged(pageSize: number, block: Block, key: number): AsyncIterable<[k: number, v: (bigint | undefined)][]>
}
