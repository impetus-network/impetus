import {sts, Block, Bytes, Option, Result, EventType, RuntimeCtx} from '../support'
import * as v9 from '../v9'

export const ruleSet =  {
    name: 'GaslessRegistry.RuleSet',
    v9: new EventType(
        'GaslessRegistry.RuleSet',
        sts.struct({
            contract: v9.H160,
            selector: sts.bytes(),
            enabled: sts.boolean(),
            minValue: sts.bigint(),
        })
    ),
}

export const ruleRemoved =  {
    name: 'GaslessRegistry.RuleRemoved',
    v9: new EventType(
        'GaslessRegistry.RuleRemoved',
        sts.struct({
            contract: v9.H160,
            selector: sts.bytes(),
        })
    ),
}
